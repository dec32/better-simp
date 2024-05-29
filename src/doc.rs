use std::fs;

use calamine::{open_workbook, Data, Range, Reader, Xlsx};
use maud::{html, Markup, DOCTYPE};
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
                span.tag { (tag) } " "
            }
            (review.comment)
        }
    )
}

// 生成单个表格
fn table(title: &str, range: Range<Data>) -> Markup {
    html!(
        div.group-title {(title)}
        @for review in range.rows().skip(1).map(parse_review).filter(Review::is_relevant) {
            div.row { (row(review)) }
        }
    )
}



// 生成文档
pub fn gen(workbook_path: &str, output_path: &str) {
    let mut workbook: Xlsx<_> = open_workbook(workbook_path).expect("打开表格失败。");
    let style = include_str!("style.css");
    let tab_1 = table("表一", workbook.worksheet_range("表一").unwrap());
    let tab_2 = table("表二", workbook.worksheet_range("表二").unwrap());
    let other = table("其他", workbook.worksheet_range("其他").unwrap());

    let markup = html!(
        (DOCTYPE)
        header {
            title {"简化字批评"}
            style { ( style )}
        }
        body {
            div.main {
                h1 {"简化字批评"}
                (tab_1)
                (tab_2)
                (other)
            }
        }
    );
    let html = markup.into_string();
    fs::write(output_path, html).unwrap()
}