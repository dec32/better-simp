use std::{collections::HashMap, fs, ops::AddAssign, path::{Path, PathBuf}};
use anyhow::{Context, Result};
use calamine::{open_workbook, Data, Range, Reader, Xlsx};
use maud::{html, Markup, DOCTYPE};
use serde_json::Value;
use crate::{parse_review, Problem, Review};


impl Review {
    fn is_relevant(&self) -> bool {
        !self.precise.is_empty() || !self.comment.is_empty()
    }
}

fn collect_reviews(range: Range<Data>) -> Vec<Review> {
    range.rows().skip(1).map(parse_review).filter(Review::is_relevant).collect()
}

fn sort_tags(tags: &mut [String], counts: &HashMap<String, u16>) {
    tags.sort_by(|t1, t2| counts[t1].cmp(&counts[t2]).reverse())
}


fn render_ids(ids: &str) -> Result<String> {
    let link = format!("IDS/{ids}.svg");
    if fs::metadata(PathBuf::from("docs").join(&link)).is_ok() {
        return Ok(link)
    }
    let mut percent_encoding = String::new();
    for byte in ids.as_bytes().iter().cloned() {
        percent_encoding.push('%');
        // 给个不带 0x 的格式化选项是会死吗？
        percent_encoding.push_str(&format!("{byte:#04X}")[2..]);
    }
    // 请求不多，字统网你忍一下……
    let url =format!("https://zi.tools/api/ids/lookupids/{percent_encoding}?replace_token");
    println!(">> {ids}");
    let resp = reqwest::blocking::get(url)?.text()?;
    let resp = serde_json::from_str::<Value>(&resp)?;
    let svg = resp.get(ids)
        .context("响应中不含 ids 数据")?
        .get("svg")
        .context("响应中不含 svg 数据")?
        .to_string();
    println!("<< {svg}");

    // svg 为一系列的用 "|" 隔开多边形的坐标，而非完整的 svg 数据
    // 另外，坐标里混杂了一些意义不明的字符（M L '），去掉才能
    // 过滤掉就是了
    let polygons = svg.split("|").map(|s|
        s.chars().filter(|char|{
            char.is_digit(10) || 
            char.is_whitespace() || 
            *char == '.' || 
            *char == ','}
        ).collect()
    ).collect::<Vec<String>>();

    let svg = html!(
        svg  
            version="1.1"
            viewBox="0 0 200 200"
            xmlns="http://www.w3.org/2000/svg"
            xmlns:cc="http://creativecommons.org/ns#"
            xmlns:dc="http://purl.org/dc/elements/1.1/"
            xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
        {
            g {
                @for points in polygons {
                    polygon points=( points ) {}
                }
            }
        }
    );
    fs::create_dir_all("docs/IDS").unwrap();
    fs::write(PathBuf::from("docs").join(&link), svg.into_string())?;
    return Ok(link)
}


// 经过渲染的表意文字序列，渲染失败时用 ? 代替
fn ids(ids: &str) -> Markup {
    match render_ids(ids) {
        Ok(path) => html!(img.glyph src = (path);),
        Err(err) => {
            println!("无法渲染「{ids}」。 {err:?}");
            html!("？")
        }
    }
}

// 表格的一行
fn row(review: Review) -> Markup {
    html!(
        div.char-cell {
            div.simp-box { 
                div.simp {(review.mapping.simp)}
                div.problem {
                    @match review.problem {
                        Problem::Major =>                    div { "" },// ⛔❌
                        Problem::Neutral | Problem::Minor => div { "🤔" }, // 🤔
                        Problem::None =>                     div { "✔️" }, // ✅✔️
                    }
                }
             }
            div.fix-box {
                div.trad { "〔" (review.mapping.trad) "〕" }
                @if review.precise.chars().next().filter(|ch|*ch != review.mapping.trad).is_some() {
                    div.fix  { 
                        "（"
                        @if review.precise.chars().count() > 1 {
                            (ids(&review.precise))
                        } @else {
                            (review.precise) 
                        }
                        "）" 
                    }
                }
            }
        }
        // 按理只需要把 .heti 作用在 span.comment 上就好了，但这样做会导致标签和文本之间发生段行
        // 原因未知
        div.comment-cell.heti {
            @for tag in review.tags {
                span.tag { (tag) }
            }
            span.comment { (review.comment) }
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

// 生成页面
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
        html {
            header {
                title { "简化字批评" }
                link rel="stylesheet" href="style.css";
                script src="script.js" {}
                script src="mojikumi.js" {}
            }
            body {
                div.main {
                    h1 { "简化字批评" }
                    div.filters {
                        @for tag in tags {
                            span.filter.tag onclick="toggle(this)" { ( tag )"("( counts[&tag] )")" }
                        }
                    }
                    (tab_1)
                    (tab_2)
                    (other)
                    div.links { a href="https://github.com/dec32/better-simp" {"GitHub"} }
                }
            }
        }
    );
    let html = markup.into_string();
    if let Some(parent) = Path::new(output_path).parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(output_path, html).unwrap()
}


#[test]
fn test_render() {
    let ids = "⿸广⿱𤰔八";
    let _ = fs::remove_file(format!("docs/IDS/{ids}.svg"));
    render_ids(ids).unwrap();
}