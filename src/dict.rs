use std::{collections::{HashMap, HashSet}, fs, ops::{AddAssign, SubAssign}, path::Path};

use calamine::{open_workbook, Reader, Xlsx};

use crate::{parse_review, CharExt, Mapping, Review, Rule};


impl Review {
    fn correct_mapping(&self) -> Mapping {
        if let Some(fix) = self.fix {
            Mapping {trad: self.mapping.trad, simp: fix}
        } else {
            self.mapping
        }
    }
}


/// 把批评批量换算为简化
fn correct_mappings(reviews: Vec<Review>) -> Vec<Mapping> {
    let mut mappings = Vec::new();
    for review in reviews {
        let mapping = review.correct_mapping();
        mappings.push(mapping)
    }
    mappings
}


/// 生成 OpenCC 映射表
pub fn gen(workbook_path: &str, output_path: &str){

    let mut workbook: Xlsx<_> = open_workbook(workbook_path).expect("打开表格失败。");
    // 非类推字
    let mut char_reviews = Vec::new();
    for row in workbook.worksheet_range("表一").unwrap().rows().skip(1) {
        char_reviews.push(parse_review(row))
    }

    // 类推字和偏旁
    let mut ichar_reviews = Vec::new();
    let mut radical_reviews = Vec::new();
    for row in workbook.worksheet_range("表二").unwrap().rows().skip(1)
        .chain(workbook.worksheet_range("其他").unwrap().rows().skip(1))
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
    for row in workbook.worksheet_range("类推").unwrap().rows().skip(1) {
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


    let mut premise = HashSet::new();
    let mut output = Vec::new();
    // 非类推字用于输出
    output.extend(correct_mappings(char_reviews));
    // 偏旁作为类推的依据
    premise.extend(correct_mappings(radical_reviews));
    // 非类推字既能用于输出，又能用于类推
    let ichar_mappings = correct_mappings(ichar_reviews);
    output.extend(ichar_mappings.as_slice());
    premise.extend(ichar_mappings.as_slice());


    // 整理类推：给每一组类推简化评分，并把当中**可用**的那些按繁字归类
    // 至少要有一个依据被用户承认才能算「可用」
    let mut derived_mappings = HashMap::new();
    let mut scores = HashMap::new();
    for rule in rules.iter() {
        if premise.contains(&rule.premise) {
            for mapping in rule.output.iter().cloned() {
                scores.entry(mapping).or_insert(0).add_assign(1);
                derived_mappings.entry(mapping.trad).or_insert_with(HashSet::new).insert(mapping);
            }
        } else {
            for mapping in rule.output.iter().cloned() {
                scores.entry(mapping).or_insert(0).sub_assign(1);
            }
        }
    }

    // 处理发生冲突的可用类推，只保留最高分的那个
    let mut derived_simps = HashMap::new();
    for (trad, mappings) in derived_mappings {
        let best_simp = mappings.into_iter().max_by(|m1, m2|scores[m1].cmp(&scores[m2])).unwrap().simp;
        derived_simps.insert(trad, best_simp);
    }

    // 固定类推：若已有简化 A->B 被定义，那么类推 A->C 被无视
    // 链式类推：若已有简化 A->B 被定义，那么类推 B->C 与类推 A->B 合并为 A->C
    let mut pinned_trads = HashSet::new();
    for mapping in output.iter_mut() {
        pinned_trads.insert(mapping.trad);
        if let Some(simpler) = derived_simps.get(&mapping.simp).cloned() {
            mapping.simp = simpler;
        };
    }

    // 把可用的类推追加到输出里（但要按照表格的顺序来）
    for rule in rules {
        if !premise.contains(&rule.premise) {
            continue;
        }
        for mapping in rule.output {
            if pinned_trads.contains(&mapping.trad) {
                continue;
            }
            if mapping.simp != derived_simps[&mapping.trad] {
                continue;
            }
            output.push(mapping)
        }
    }

    // 输出到文件
    let mut text = String::with_capacity(output.len() * 10);
    let mut dup = HashMap::new();
    for mapping in output {
        // OpenCC 不允许重复项
        if let Some(prev) = dup.insert(mapping.trad, mapping.simp) {
            if prev != mapping.simp {
                println!("检测到冲突「{}{{{}|{}}}」", mapping.trad, prev, mapping.simp)
            }
            continue;
        }
        // 拋弃形如 X -> X 的映射规则（留到此处才删除是因为 X -> X 可能有「抑制」类推的用意）
        if mapping.trad == mapping.simp {
            continue;
        }
        text.push(mapping.trad);
        text.push('\t');
        text.push(mapping.simp);
        text.push('\n');
    }
    if let Some(parent) = Path::new(output_path).parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(output_path, text).unwrap();
}