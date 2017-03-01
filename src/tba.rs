use curl::easy::{Easy, List};
use std::str;

pub struct Response {
    pub code: u32,
    pub data: Vec<u8>,
    pub last_modified: String,
}

pub fn request(url_ext: &str, date: &str) -> Response {
    let request_url = format!("https://www.thebluealliance.com/api/v2/{}", url_ext);
    //let last_time = last_checked(url_ext);
    //let time = DateTime::<UTC>::from_utc(NaiveDateTime::from_timestamp(last_time as i64, 0), UTC);
    let mut easy = Easy::new();
    let mut list = List::new();
    let mut data = Vec::new();
    list.append("X-TBA-App-Id: Carl Colglazier:FRC ELO:0.0.0").unwrap();
    if date.len() > 0 {
        let time_header = format!("If-Modified-Since: {}", date);
        list.append(&time_header).unwrap();
    }
    easy.http_headers(list).unwrap();
    easy.url(&request_url).unwrap();
    let mut headers = String::new();
    {
        let mut transfer = easy.transfer();
        transfer.write_function(|new_data| {
            data.extend_from_slice(new_data);
            Ok(new_data.len())
        }).unwrap();
        transfer.header_function(|header| {
            let s = str::from_utf8(header).unwrap().to_string();
            if s.starts_with("Last-Modified: ") {
                headers.push_str(&s[15..]);
            }
            true
        }).unwrap();
        transfer.perform().unwrap();
    }
    let code = easy.response_code().unwrap();
    return Response {
        code: code,
        data: data,
        last_modified: headers,
    };
}
