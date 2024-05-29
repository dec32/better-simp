use std::{collections::HashMap, fs, ops::AddAssign};

use calamine::{open_workbook, Data, Range, Reader, Xlsx};
use maud::{html, Markup, PreEscaped, DOCTYPE};
use crate::{parse_review, Review};


impl Review {
    fn is_relevant(&self) -> bool {
        !self.precise.is_empty() || !self.comment.is_empty()
    }
}

// 表格的一行
fn row(review: Review) -> Markup {
    html!(
        div.entry {
            div.simp { (review.mapping.simp) }
            div {
                div.trad { "〔"(review.mapping.trad)"〕" }
                @if review.precise.chars().next().filter(|ch|*ch != review.mapping.trad).is_some() {
                    @if review.precise.chars().count() > 1 {
                        // TODO: render IDS
                        div.fix  { "（？）"}
                    } @else {
                        div.fix  { "（" (review.precise) "）"}
                    }
                }
            }
        }
        div.comment {
            @for tag in review.tags {
                span.tag { (tag) }
            }
            (review.comment)
        }
    )
}

// 生成单个表格
fn table(title: &str, reviews: Vec<Review>) -> Markup {
    html!(
        div.group-title {(title)}
        @for review in reviews {
            div.row { (row(review)) }
        }
    )
}

fn collect_reviews(range: Range<Data>) -> Vec<Review> {
    range.rows().skip(1).map(parse_review).filter(Review::is_relevant).collect()
}


fn sort_tags(tags: &mut [String], counts: &HashMap<String, u16>) {
    tags.sort_by(|t1, t2| counts[t1].cmp(&counts[t2]).reverse())
}

// 生成文档
pub fn gen(workbook_path: &str, output_path: &str) {
    let mut workbook: Xlsx<_> = open_workbook(workbook_path).expect("打开表格失败。");

    let mut tab_1 = collect_reviews(workbook.worksheet_range("表一").unwrap());
    let mut tab_2 = collect_reviews(workbook.worksheet_range("表二").unwrap());
    let mut other = collect_reviews(workbook.worksheet_range("其他").unwrap());

    // 把标签按频度排序
    let mut counts = HashMap::new();
    for review in tab_1.iter().chain(tab_2.iter()).chain(other.iter()) {
        for tag in review.tags.iter() {
            // clone goes brrrrrrrr
            counts.entry(tag.clone()).or_insert(0).add_assign(1);       
        }
    }

    let mut tags = counts.keys().cloned().collect::<Vec<_>>();
    sort_tags(&mut tags, &counts);
    for review in tab_1.iter_mut().chain(tab_2.iter_mut()).chain(other.iter_mut()) {
        sort_tags(&mut review.tags, &counts)
    }

    // 生成页面
    let tab_1 = table("表一", tab_1);
    let tab_2 = table("表二", tab_2);
    let other = table("其他", other);

    let markup = html!(
        (DOCTYPE)
        header {
            title {"简化字批评"}
            // link rel="stylesheet" href="https://unpkg.com/heti/umd/heti.min.css";
            // script src="https://unpkg.com/heti/umd/heti-addon.min.js";
            // script {
            //     "const heti = new Heti('.heti');heti.autoSpacing();"
            // }
            script { ( PreEscaped(include_str!("script.js")) ) }
            style { ( PreEscaped(include_str!("style.css")) ) }
        }
        body {
            div.main {
                h1 {"简化字批评"}
                div.filters {
                    @for tag in tags {
                        span.filter.tag onclick="toggle(this)" { ( tag )"("( counts[&tag] )")" }
                    }
                }
                (tab_1)
                (tab_2)
                (other)
            }
        }
    );
    let html = markup.into_string();
    fs::write(output_path, html).unwrap()
}