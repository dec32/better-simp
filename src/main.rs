use std::{env, fs, path::PathBuf};

use calamine::Data;
use getargs::{Arg, Options};

mod doc;
mod dict;

/// 表示一组简化和对其的订正
#[derive(Clone)]
struct Review {
    // 供字典生成用
    mapping: Mapping,
    fix: Option<char>,
    // 更详细的内容，供文档生成用
    precise: String,
    problem: Problem,
    tags: Vec<String>,
    comment: String,
}


// 表示一则简化的问题有多大
#[derive(Clone, PartialEq, Eq)]
enum Problem {
    Major,   // 问题很大，必须改
    Neutral, // 问题不大，但我就是要改（用叹号标记）
    Minor,   // 问题不大
    None,    // 没问题
}


/// 表示一组简化
#[derive(PartialEq, Eq, Hash, Clone, Copy)]
struct Mapping {
    trad: char,
    simp: char,
}


/// 类推规则
struct Rule {
    premise: Mapping,
    output: Vec<Mapping>
}

/// 把一行数据翻译为一则批评
fn parse_review(row: &[Data]) -> Review {
    let mapping = Mapping {
        trad: row[0].to_string().chars().next().expect(&format!("解析异常：{:?}", row)),
        simp: row[1].to_string().chars().next().expect(&format!("解析异常：{:?}", row)),
    };

    let mut precise = row[2].to_string();
    let compatible = row[3].to_string().chars().next();
    let tags = row[4].to_string().split(char::is_whitespace)
        .filter(|s|!s.is_empty())
        .map(String::from)
        .collect::<Vec<_>>();
    let comment = {
        // 补一下懒得写的句号
        let mut comment = row[5].to_string();
        if let Some(last) = comment.chars().last() {
            if !matches!(last, '。' | '？' | '！') {
                comment.push('。');
            }
        }
        comment
    };

    // TODO 校验 presice 字符串的格式
    let (problem, fix) = match precise.chars().last() {
        None =>       (Problem::None,    None),
        Some('？') => (Problem::Minor,   None),
        Some('！') => (Problem::Neutral, compatible.or(precise.chars().next())),
        Some(_) =>    (Problem::Major,   compatible.or(precise.chars().next()))
    };

    if matches!(problem, Problem::Minor | Problem::Neutral) {
        precise.pop();
    }

    Review {mapping, fix, precise, problem, tags, comment}
}


/// 偏旁只做简化依据，本身不构成简化规则
trait CharExt {
    fn is_radical(self) -> bool;
}

impl CharExt for char {
    fn is_radical(self) -> bool {
        matches!(self, '訁'|'飠'|'糹'|'𤇾'|'𰯲'|'釒'|'𦥯'|'䜌'|'睪'|'巠'|'咼'|'昜'|'臤'|'戠')
    }
}


fn main() {
    let mut workbook_path = "./简化字批评.xlsx";
    let mut output_path = "./TSCharacters.txt";
    let mut doc = false;

    let args = env::args().skip(1).collect::<Vec<_>>();
    let args = args.iter().map(String::as_str);
    let mut opts = Options::new(args);
    while let Some(opt) = opts.next_arg().expect("无法解析命令行参数。") {
        match opt {
            Arg::Long("rime") | Arg::Short('r') => {
                let opencc_path = PathBuf::from(env::var_os("APPDATA").unwrap()).join("rime/opencc");
                output_path = opencc_path.join("TPCharacters.txt").to_string_lossy().to_string().leak();
                fs::write(opencc_path.join("t2p.json"), include_str!("../t2p.json")).unwrap();
            }
            Arg::Long("doc") | Arg::Short('d') => {
                doc = true;
            }
            Arg::Long("input") | Arg::Short('i')=> {
                workbook_path = opts.value_opt().expect("获取输入路径时发生异常。")
            }
            Arg::Long("output") | Arg::Short('o') => {
                output_path = opts.value_opt().expect("获取输出路径时发生异常。")
            }
            _ => {
                println!("使用: ");
                println!("  simp [--input <表格路径>][--output <输出路径>][--rime]");
                println!("说明: ");
                println!("  --input:  表格路径，默认为 ./简化字批评.xlsx");
                println!("  --output: 输出路径，默认为 ./TSCharacters.txt");
                println!("  --rime:   输出到 %APPDATA%/rime/opencc 供 RIME 使用");
                return;
            }
        }
    }

    if doc {
        output_path = "./docs/index.html";
        doc::gen(workbook_path, output_path);
    } else {
        dict::gen(workbook_path, output_path);
    }
}




