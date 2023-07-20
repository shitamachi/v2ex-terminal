use anyhow::Result;
use chrono::{DateTime, FixedOffset};
use ratatui::widgets::ListItem;
use scraper::Html;

#[derive(Debug, Clone)]
pub struct V2exNode {
    name: String,
    sub_url: String,
}

#[derive(Debug, Clone)]
pub struct V2exUser {
    name: String,
    sub_url: String,
}

#[derive(Debug, Clone)]
pub struct V2exTopic {
    id: i32,
    title: String,
    short_url: String,
    node: V2exNode,
    send_user: V2exUser,
    send_time: DateTime<FixedOffset>,
    last_reply_user: V2exUser,
}

impl V2exTopic {
    pub fn list_item_format(&self) -> String {
        let s = format!("title: {} time: {}", self.title, self.send_time.format("%Y/%m/%d %H:%M"));
        s
    }

    pub fn get_topic_url(&self) -> String {
        format!("https://www.v2ex.com{}", self.short_url)
    }
}

impl<'a> From<&V2exTopic> for ListItem<'a> {
    fn from(v: &V2exTopic) -> Self {
        let s = format!("title: {} time: {}", v.title, v.send_time.format("%Y/%m/%d %H:%M"));
        ListItem::new(s)
    }
}

pub async fn get_v2ex_page(page: i32) -> Result<String> {
    let client = reqwest::Client::builder()
        .proxy(reqwest::Proxy::all("http://127.0.0.1:7890")?)
        .build()?;
    let resp = client.get(format!("https://www.v2ex.com/?p={}", page)).send().await?;
    let s = resp.bytes().await.map_err(|e| e.into()).and_then(|bytes| {
        String::from_utf8(bytes.to_vec()).map_err(|e| e.into())
    });
    s
}

pub fn parse_v2ex_page(page: String) -> Result<Vec<V2exTopic>> {
    let document = Html::parse_document(&page);
    let mut vec = Vec::new();
    // all class is "cell item" node
    for node in document.select(&scraper::Selector::parse(".cell.item").unwrap()) {
        // get class is "topic-link" a node
        let a = node.select(&scraper::Selector::parse(".topic-link").unwrap()).next().unwrap();
        // get a node href link
        let href = a.value().attr("href").unwrap();
        // get a node text
        let text = a.inner_html();
        // get span node of class is "topic_info"

        let topic_info_span = node.select(&scraper::Selector::parse(".topic_info").unwrap()).next().unwrap();
        let (v2ex_node, send_user, send_time, last_reply_user) = parse_v2ex_cell_item_topic_info(topic_info_span)?;


        // example: /t/958255#reply0
        let topic_id = href.split('/').nth(2).and_then(|s|
            s.split('#').next().and_then(|s| s.parse::<i32>().ok())
        );

        let v2ex_topic = V2exTopic {
            title: text,
            short_url: href.to_string(),
            id: topic_id.unwrap_or(0),
            node: v2ex_node,
            send_user,
            send_time,
            last_reply_user,
        };
        vec.push(v2ex_topic);
    }

    Ok(vec)
}

// topic_info:
// a tag, class is "node", v2ex topic node info
// strong tag has a tag inner, first strong tag, node send user info; href is user sub url, text is user name
// span tag, have title attr, node send time info; title format is "2023-07-19 19:15:44 +08:00", text is "1 小时 29 分钟前"
// strong tag has a tag inner, second strong tag, node send reply info; href is user sub url, text is user name
fn parse_v2ex_cell_item_topic_info(topic_info_span: scraper::ElementRef) -> Result<(V2exNode, V2exUser, DateTime<FixedOffset>, V2exUser)> {
    let all_strong_nodes = topic_info_span.select(&scraper::Selector::parse("strong").unwrap()).collect::<Vec<_>>();
    let node_a = topic_info_span.select(&scraper::Selector::parse("a").unwrap()).next().unwrap();
    let node_strong = all_strong_nodes.first().unwrap();
    let node_span = topic_info_span.select(&scraper::Selector::parse("span").unwrap()).next().unwrap();
    let node_strong2 = all_strong_nodes.last().unwrap();

    // v2ex node
    let node_a_href = node_a.value().attr("href").unwrap();
    let node_a_text = node_a.inner_html();

    let v2ex_node = V2exNode {
        name: node_a_text,
        sub_url: node_a_href.to_string(),
    };

    // send user
    let inner_a = node_strong.select(&scraper::Selector::parse("a").unwrap()).next().unwrap();
    let send_user = V2exUser {
        name: inner_a.inner_html(),
        sub_url: inner_a.value().attr("href").unwrap().to_string(),
    };

    // send time
    let send_time = chrono::DateTime::parse_from_str(node_span.value().attr("title").unwrap(), "%Y-%m-%d %H:%M:%S %z")?;

    // last reply user
    let inner_a = node_strong2.select(&scraper::Selector::parse("a").unwrap()).next().unwrap();
    let last_reply_user = V2exUser {
        name: inner_a.inner_html(),
        sub_url: inner_a.value().attr("href").unwrap().to_string(),
    };

    Ok((v2ex_node, send_user, send_time, last_reply_user))
}

mod test {
    #[tokio::test]
    async fn test_get_and_parse_v2ex_page() {
        let page = super::get_v2ex_page(1).await.unwrap();
        let vec = super::parse_v2ex_page(page).unwrap();
        println!("{:#?}", vec);
    }
}