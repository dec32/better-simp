use std::{collections::HashSet, env};

use calamine::{open_workbook, Data, Reader, Xlsx};

/// 表示一组简化和对其的批评。Critique 这个词太难打了，所以使用 Review。
#[derive(Clone, Copy)]
struct Review {
    mapping: Mapping,
    fix: Option<char>
}

impl Review {
    fn derive_mapping(&self) -> Mapping {
        if let Some(fix) = self.fix {
            Mapping {trad: self.mapping.trad, simp: fix}
        } else {
            self.mapping
        }
    }
}

/// 表示一组简化
#[derive(PartialEq, Eq, Hash, Clone, Copy)]
struct Mapping {
    trad: char,
    simp: char,
}

/// 类推规则
struct Infer {
    premise: Mapping,
    output: Vec<Mapping>
}

/// 把一行数据翻译为一则批评
fn parse_review(row: &[Data]) -> Review {
    let mapping = Mapping {
        trad: row[0].to_string().chars().next().unwrap(),
        simp: row[1].to_string().chars().next().unwrap(),
    };

    let precise = row[2].to_string();
    let compatible = row[3].to_string().chars().next();

    if precise.is_empty() || precise.ends_with("？") {
        Review { mapping, fix: None }
    } else {
        // TODO 校验 presice 字符串的格式
        Review { mapping, fix: compatible.or(precise.chars().next())}
    }
}



/// 把批评批量换算为简化，且按需过滤掉不需要的映射规则
fn derive_mappings(reviews: Vec<Review>, patch: bool) -> Vec<Mapping> {
    let mut mappings = Vec::new();
    for review in reviews {
        let mapping = review.derive_mapping();
        // 在补丁中，不需要重复原规则
        if patch && mapping == review.mapping {
            continue;
        }
        // 在完整映射表中，形如 X -> X 的映射规则是多余的
        if !patch && mapping.trad == mapping.simp {
            continue;
        }
        mappings.push(mapping)
    }
    mappings
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

/// 生成 OpenCC 映射表
fn gen(char_reviews: Vec<Review>, ichar_reviews: Vec<Review>, radical_reviews: Vec<Review>, infers: Vec<Infer>, patch: bool) {
    let mut premise = HashSet::new();

    let mut output = Vec::new();

    // 非类推字用于输出
    output.extend(derive_mappings(char_reviews, patch));

    // 偏旁作为类推的依据
    premise.extend(derive_mappings(radical_reviews, patch));

    // 非类推字既能用于输出，又能用于类推
    let ichar_mappings = derive_mappings(ichar_reviews, patch);
    output.extend(ichar_mappings.as_slice());
    premise.extend(ichar_mappings.as_slice());


    // 输出满足前提的类推
    // TODO 有以下三个问题没有解决：
    // 递归类推：类推得到的映射再次作为类推的依据（故需把繁字作为 key）
    // 链式类推：映射的简字可根据其他依据进行再次类推（故需把简字作为 key）
    // 多因类推：由多条类推规则共同生成的类推
    // 另外还要想办法保留插入顺序（故不能用 HashMap）
    for infer in infers {
        if premise.contains(&infer.premise) {
            output.extend(infer.output)
        }
    }
    
    for mapping in output {
        println!("{}\t{}", mapping.trad, mapping.simp)
    }
}


fn main() {

    let path = env::args().nth(1).unwrap_or("./简化字批评.xlsx".to_string());
    let patch = false;

    let mut workbook: Xlsx<_> = open_workbook(path).unwrap();

    // 非类推字
    let mut char_reviews = Vec::new();
    for row in workbook.worksheet_range("表一").unwrap().rows().skip(1)
        .chain(workbook.worksheet_range("其他").unwrap().rows().skip(1))
        .chain(workbook.worksheet_range("增补").unwrap().rows().skip(1))
    {
        char_reviews.push(parse_review(row))
    }


    // 类推字和偏旁
    let mut ichar_reviews = Vec::new();
    let mut radical_reviews = Vec::new();
    for row in workbook.worksheet_range("表二").unwrap().rows().skip(1)
    {
        let review = parse_review(row);
        if review.mapping.trad.is_radical() {
            radical_reviews.push(review);
        } else {
            ichar_reviews.push(parse_review(row))
        }
    }


    // 类推规则
    let mut infers = Vec::new();
    for row in workbook.worksheet_range("类推").unwrap().rows().skip(1) {
        let premise = Mapping {
            trad: row[0].to_string().chars().next().unwrap(),
            simp: row[1].to_string().chars().next().unwrap(),
        };

        // TODO：含有多个可类推部件的汉字只类推一个部件时所得到结果可能需要用 IDS 表达。
        // char 无法储存 IDS，故 Mapping 也无法记录部分类推
        let mut output = Vec::new();
        let string = row[2].to_string();
        let mut chars = string.chars();
        loop {
            let Some(ch) = chars.next() else {
                break;
            };
            if ch.is_whitespace() {
                continue;
            }
            output.push(Mapping {
                trad: ch,
                simp: chars.next().expect(&format!("类推「{}{}」中「{}」缺少对应的简化字", premise.trad, premise.simp, ch))
            })
        }
        if !output.is_empty() {
            infers.push(Infer{ premise, output })
        }
    }

    gen(char_reviews, ichar_reviews, radical_reviews, infers, patch);
}




