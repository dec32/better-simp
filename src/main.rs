use std::{collections::{HashMap, HashSet}, env, fs, hash::Hash, ops::{AddAssign, SubAssign}};

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
struct Rule {
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
fn derive_mappings(reviews: Vec<Review>) -> Vec<Mapping> {
    let mut mappings = Vec::new();
    for review in reviews {
        let mapping = review.derive_mapping();
        // 形如 X -> X 的映射规则是多余的
        if mapping.trad == mapping.simp {
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
fn gen(char_reviews: Vec<Review>, ichar_reviews: Vec<Review>, radical_reviews: Vec<Review>, rules: Vec<Rule>) {
    let mut premise = HashSet::new();
    let mut output = Vec::new();
    // 非类推字用于输出
    output.extend(derive_mappings(char_reviews));
    // 偏旁作为类推的依据
    premise.extend(derive_mappings(radical_reviews));
    // 非类推字既能用于输出，又能用于类推
    let ichar_mappings = derive_mappings(ichar_reviews);
    output.extend(ichar_mappings.as_slice());
    premise.extend(ichar_mappings.as_slice());


    // 整理类推：给每一组类推简化评分，并把当中**可用**的那些按繁字归类
    // 至少要有一个依据被用户承认才能算「可用」
    let mut trad_to_mappings = HashMap::new();
    let mut scores = HashMap::new();
    for rule in rules.iter() {
        if premise.contains(&rule.premise) {
            for mapping in rule.output.iter().cloned() {
                scores.entry(mapping).or_insert(0).add_assign(1);
                trad_to_mappings.entry(mapping.trad).or_insert_with(HashSet::new).insert(mapping);
            }
        } else {
            for mapping in rule.output.iter().cloned() {
                scores.entry(mapping).or_insert(0).sub_assign(1);
            }
        }
    }

    // 处理发生冲突的可用类推，只保留最高分的那个
    let mut trad_to_mapping = HashMap::new();
    for (trad, mappings) in trad_to_mappings {
        let mut mappings = mappings.into_iter();
        let mut best_mapping = mappings.next().unwrap();
        let mut best_score = scores[&best_mapping];
        for mapping in mappings {
            let score = scores[&mapping];
            if score > best_score {
                best_mapping = mapping;
                best_score = score;
            }
        }
        trad_to_mapping.insert(trad, best_mapping);
    }

    // 固定类推：若已有简化 A->B 被定义，那么类推 A->C 被无视
    // 链式类推：若已有简化 A->B 被定义，那么类推 B->C 与类推 A->B 合并为 A->C
    let mut pinned_trads = HashSet::new();
    for mapping in output.iter_mut() {
        pinned_trads.insert(mapping.trad);
        if let Some(chain) = trad_to_mapping.get(&mapping.simp) {
            mapping.simp = chain.simp;
        };
    }

    // 把可用的类推追加到输出里
    for rule in rules {
        if !premise.contains(&rule.premise) {
            continue;
        }
        for mapping in rule.output {
            if trad_to_mapping[&mapping.trad] != mapping {
                continue;
            }
            if pinned_trads.contains(&mapping.trad) {
                continue;
            }
            output.push(mapping);
        }
    }


    let mut text = String::with_capacity(output.len() * 10);
    for mapping in output {
        text.push(mapping.trad);
        text.push('\t');
        text.push(mapping.simp);
        text.push('\n');
    }
    print!("{text}");
    fs::write("./TSCharacters.txt", text).unwrap()
}


fn main() {

    let path = env::args().nth(1).unwrap_or("./简化字批评.xlsx".to_string());
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
    let mut rules = Vec::new();
    for row in workbook.worksheet_range("表三").unwrap().rows().skip(1) {
        let premise = Mapping {
            trad: row[0].to_string().chars().next().unwrap(),
            simp: row[1].to_string().chars().next().unwrap(),
        };

        let mut output = Vec::new();
        let row_2 = row[2].to_string();
        let row_3 = row[3].to_string();
        let mut chars = row_2.chars().chain(row_3.chars());
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
            rules.push(Rule{ premise, output })
        }
    }

    gen(char_reviews, ichar_reviews, radical_reviews, rules);
}




