use std::fs;
use curl::easy::{Easy, List};
use filetime::FileTime;
use chrono::prelude::*;
use chrono::naive::datetime::NaiveDateTime;
use chrono::DateTime;
use std::io::prelude::*;
use std::fs::File;
use std::path::Path;
use std::fs::OpenOptions;

pub const TBA_DATA_DIR: &'static str = "./tba_data";

/// Find the last time that the request has been made based on the cache files.
/// Returns 0 if no such file exists.
fn last_checked(url_ext: &str) -> u64 {
    let local_file_path_str = format!("{}/{}", TBA_DATA_DIR, url_ext);
    let metadata = match fs::metadata(local_file_path_str) {
        Ok(meta) => meta,
        Err(_) => return 0,
    };
    let modified_time = FileTime::from_last_modification_time(&metadata);
    // NOTE: This is Linux specific. Time will be different on Windows!
    return modified_time.seconds();
}

fn write_file(url_ext: &str, output: &[u8]) {
    let path_str = format!("{}/{}", TBA_DATA_DIR, url_ext);
    let path = Path::new(&path_str);
    if !path.exists() {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
    }
    let buffer = File::create(&path_str);
    buffer.unwrap().write(output);
    OpenOptions::new().write(true).append(true).create(true)
        .open("temp.txt").unwrap().write(format!("{}\n", path_str).as_bytes());
}

pub fn request(url_ext: &str) -> Vec<u8> {
    let request_url = format!("https://www.thebluealliance.com/api/v2/{}", url_ext);
    let last_time = last_checked(url_ext);
    let time = DateTime::<UTC>::from_utc(NaiveDateTime::from_timestamp(last_time as i64, 0), UTC);
    let mut easy = Easy::new();
    let mut list = List::new();
    let mut data = Vec::new();
    list.append("X-TBA-App-Id: Carl Colglazier:FRC ELO:0.0.0").unwrap();
    if last_time > 0 {
        let time_header = format!("If-Modified-Since: {}", time.to_rfc2822());
        list.append(&time_header).unwrap();
        println!("{}", time_header);
    }
    easy.http_headers(list).unwrap();
    easy.url(&request_url).unwrap();
    {
        let mut transfer = easy.transfer();
        transfer.write_function(|new_data| {
            data.extend_from_slice(new_data);
            Ok(new_data.len())
        }).unwrap();
        transfer.perform().unwrap();
    }
    let code = easy.response_code().unwrap();
    if code == 200 {
        write_file(url_ext, &data);
        println!("Updating {}", url_ext);
    } else {
        data.clear();
    }
    return data;
}
