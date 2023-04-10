use std::time;
use std::{fs, io::BufRead, collections::HashMap};
mod mapping;
use quick_xml;
const FILE: &str = "../../english.stackexchange.com/Posts.xml";

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum ExclusiveData{
    Question{title:String,accepted_answer_id: Option<u32>,answer_count:u32},
    Answer{parent_id:u32,}
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Row {
    id: u32,
    post_type_id: u8,
    score: i32,
    view_count: u32,
    body: String,
    data: ExclusiveData,
}

fn parse_row(s: &str) -> Option<Row> {
    use quick_xml::{events::*, reader::*};
    let mut reader = Reader::from_str(s);
    reader.trim_text(true);
    let mut id = 0;
    let mut post_type_id = 0;
    let mut score = 0;
    let mut view_count = 0;
    let mut title = String::new();
    let mut accepted_answer_id = None;
    let mut parent_id = 0;
    let mut body = String::new();
    let mut answer_count = 0;
    loop {
        let event_read = reader.read_event().ok()?; 
        match event_read{
            Event::Empty(x) => {
                for attr in x.attributes().filter_map(|x| x.ok()) {
                    match attr.key.local_name().as_ref() {
                        b"Id" => id = attr.unescape_value().ok()?.into_owned().parse().ok()?,
                        b"PostTypeId" => {
                            post_type_id = attr.unescape_value().ok()?.into_owned().parse().ok()?
                        }
                        b"Score" => {
                            score = attr.unescape_value().ok()?.into_owned().parse().ok()?
                        }
                        b"ViewCount" => {
                            view_count = attr.unescape_value().ok()?.into_owned().parse().ok()?
                        }
                        b"AnswerCount" => {
                            answer_count = attr.unescape_value().ok()?.into_owned().parse().ok()?
                        }
                        b"Title" => title = attr.unescape_value().ok()?.into_owned().parse().ok()?,
                        b"Body" => body = attr.unescape_value().ok()?.into_owned().parse().ok()?,
                        b"ParentId" => parent_id = attr.unescape_value().ok()?.into_owned().parse().ok()?,
                        b"AcceptedAnswerId" => {
                            accepted_answer_id =
                                attr.unescape_value().ok()?.into_owned().parse().ok()
                        }
                        _ => continue,
                    }
                }
            }
            _ => {
                break
            },
        }
    }
    let body = dissolve::strip_html_tags(&body).join(""); 
    let title = dissolve::strip_html_tags(&title).join(""); 
    if post_type_id==1{ //question
        let data = ExclusiveData::Question { title , accepted_answer_id , answer_count };
        return Some(Row {
            id,
            post_type_id,
            score,
            body,
            view_count,
            data
        })
    }
    let data = ExclusiveData::Answer { parent_id };
    return Some(Row {
        id,
        post_type_id,
        score,
        body,
        view_count,
        data
    })
}
fn save_to_csv(answers: &HashMap<u32,Row>, questions: &HashMap<u32,Row>) -> Option<()> {
    let mut file = format!("./{PREFIX}-saved-unknown_time.csv").into();
    if let Ok(v) =  time::SystemTime::UNIX_EPOCH.elapsed() {
        file = format!("./{PREFIX}-saved-{}.csv",v.as_secs());
    };
    let mut writer = csv::Writer::from_path(file).ok()?;
    writer.write_record(&["title","question","answer"]).ok()?;
    let mut parents_lost = 0u32;
    let mut encountered_unreachable = 0u32;
    for answer in answers.values(){
        //println!("answer = {answers:#?}");
        let data = answer.data.clone();
        match &data{
            ExclusiveData::Question { title: _, accepted_answer_id: _, answer_count: _ } => {
                //println!("ENCOUNTERED UNREACHABLE");
                encountered_unreachable += 1;
                continue;   
            },
            ExclusiveData::Answer { parent_id } => {
                let q = if let Some(v) = questions.get(&parent_id) {v.clone()}  else {
                    //println!("PARENT NOT FOUND");
                    parents_lost += 1;
                    continue;
                };
                let data = q.data;
                match &data{
                    ExclusiveData::Question { title, accepted_answer_id: _, answer_count: _ } => {
                        if title.len()<10 || answer.body.len()<10{
                            continue;
                        }

                        let title = title.trim();
                        let question = q.body.clone();
                        let answer = answer.body.clone();

                        let question = question.trim();
                        let answer = answer.trim();
                        writer.write_record(&[title,question,answer]).ok()?;
                    },
                    ExclusiveData::Answer { parent_id: _ } => {
                        //println!("ENCOUNTERED UNREACHABLE");
                        encountered_unreachable += 1;
                        continue;   
                    },
                };
            },
        }
    }
    println!("total parents not found: {}",parents_lost);
    println!("total unreachable encounters: {}",encountered_unreachable);
    return Some(())
}

const PREFIX: &str = "min-20-score";

fn main() {
    let file = fs::File::open(FILE).unwrap();
    let mut iterator = std::io::BufReader::new(file).lines().filter_map(|x| x.ok());
    iterator.next();
    iterator.next();
    let mut answers: HashMap<u32,Row> = HashMap::new();
    let mut questions: HashMap<u32,Row> = HashMap::new(); 
    for line in iterator {
        if line.starts_with("<") {
            continue;
        }
        let row = if let Some(x) = parse_row(&line) {
            x
        } else {
            continue;
        };
        if row.score < 20{
            continue;   
        }
        //if row.post_type_id > 2 || row.post_type_id == 0{continue;}
        if row.post_type_id == 1 { // question
            questions.insert(row.id, row); 
        } else if row.post_type_id == 2 { // answer
            answers.insert(row.id, row);
        }
    }
    save_to_csv(&answers, &questions);
    println!("Hello, world!");
}
