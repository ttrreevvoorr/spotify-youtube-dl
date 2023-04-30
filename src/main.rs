//use base64;
use reqwest::Client;
//use reqwest::Client;
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, ACCEPT_ENCODING, ACCEPT_LANGUAGE, USER_AGENT};
use scraper::{Html, Selector};
use serde_json::Value;
use std::error::Error;
use std::fs;
use async_std::main;

//use duktape::{Context, Object};
/*
#[derive(Debug)]
struct Track {
  title: String,
  artist: String,
  //Option<album>: String,
  //dOption<cover>: String
}
*/

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let url = "https://open.spotify.com/album/6hPkbAV3ZXpGZBGUvL6jVM"; // Replace with your desired Spotify URL
    let page_data = get_spotify_data(&url).await?;
    //println!("{:?}", page_data);

    let page_tracks = get_tracks(&page_data);
    //println!("{:?}", page_tracks);

    let binding = &page_tracks.unwrap();
    let track_data = get_track_data(binding).await;
    println!("{:?}", &track_data);
 
    Ok(())
}

async fn get_spotify_data(url: &str) -> Result<Html,Box<dyn Error>> {
    let mut headers = HeaderMap::new();
    headers.insert(
        USER_AGENT,
        HeaderValue::from_static("Mozilla/5.0 (Macintosh; Intel Mac OS X x.y; rv:42.0) Gecko/20100101 Firefox/42.0")
    );
    headers.insert(ACCEPT, HeaderValue::from_static("text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.9"));
    //headers.insert(ACCEPT_ENCODING, HeaderValue::from_static("gzip, deflate, br"));
    headers.insert(ACCEPT_LANGUAGE, HeaderValue::from_static("en-US,en;q=0.9"));

    let client = Client::builder().default_headers(headers).build()?;
    let resp = client.get(url).send().await?;
    let html = resp.text().await.unwrap();

    let parsed_html = Html::parse_document(&html);
    //println!("{:?}", html);
    return Ok(parsed_html);
}

async fn get_track_data(tracks: &Vec<String>) -> Result<Vec<String>, Box<dyn Error>> {
    //let track_data = get_spotify_data(&tracks[0]);
    //println!("{:?}", track_data)
    let mut track_list = Vec::new();
    let meta_selector = Selector::parse("meta").unwrap();
    for track in tracks {
        let track_html = get_spotify_data(&track).await?;
        let meta_elements = track_html.select(&meta_selector).collect::<Vec<_>>();
        
        let mut title:String = "".to_string();
        let mut artist:String = "".to_string();
        //let mut album:String;

        for element in meta_elements {
            if let (Some(content), Some(property)) = (element.value().attr("content"), element.value().attr("property")) {
                if property == "og:title" {
                    title = content.to_string();
                }
                else if property == "og:description" {
                    artist = content.split("·").next().unwrap().trim().to_string();
                }
            }
        }
        if !artist.is_empty() && !title.is_empty() {
            let query = format!("{} {} {}", title, artist, "audio");
            let url = get_youtube_url(query).await?;
            
            track_list.push(url);
        }
    }
    return Ok(track_list);
}

fn get_tracks(parsed_html: &Html) -> Result<Vec<String>, Box<dyn Error>> {
    let meta_selector = Selector::parse("meta").unwrap();
    let meta_elements = parsed_html.select(&meta_selector).collect::<Vec<_>>();
    
    let mut tracks = Vec::new();
    for element in meta_elements {
        if let (Some(content), Some(name)) = (element.value().attr("content"), element.value().attr("name")) {
            if name == "music:song" {
                tracks.push(content.to_string());
            }
        }
    }
    return Ok(tracks);
}


async fn get_youtube_url(query: String) -> Result<String, Box<dyn std::error::Error>> {
    let mut headers = HeaderMap::new();
    headers.insert(
        USER_AGENT,
        HeaderValue::from_static("Mozilla/5.0 (Macintosh; Intel Mac OS X x.y; rv:42.0) Gecko/20100101 Firefox/42.0")
    );
    headers.insert(ACCEPT, HeaderValue::from_static("text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.9"));
    //headers.insert(ACCEPT_ENCODING, HeaderValue::from_static("gzip, deflate, br"));
    headers.insert(ACCEPT_LANGUAGE, HeaderValue::from_static("en-US,en;q=0.9"));

    let client = Client::builder().default_headers(headers).build()?;
    let response = client.get(&url).send().await?;
    let html = response.text().await.unwrap();

    //let html = response.text()?;
    let parsed_html = Html::parse_document(&html);
    
    let selector = Selector::parse("script").unwrap();
    //let mut video_urls = Vec::new();

    //let urls: Vec<String> = parsed_html
    //    .select(&selector)
    //    .map(|element| element.value().attr("href").unwrap().to_string())
    //    .collect();

    //println!("{:#?}", urls);

    let variable_name = "ytInitialData";
 
     let mut video_url: String = "".to_string();

     for element in parsed_html.select(&selector) {

        let script_text = element.text().collect::<Vec<_>>().join("");
        if script_text.contains(&format!("var {} = ", variable_name)) {

            let start_delimiter = '{';
            let end_delimiter = '}';

            // Find the starting index of the JSON object
            let start_index = match script_text.find(start_delimiter) {
                Some(script_text) => script_text,
                None => todo!()
                //None => return, // or handle the error
            };

            // Find the ending index of the JSON object
            let end_index = match script_text.rfind(end_delimiter) {
                Some(script_text) => script_text,
                None => todo!()
                //None => return, // or handle the error
            };

            let json_str = &script_text[start_index..=end_index];

            let parsed: Value = serde_json::from_str(&json_str)?;

            if let Some(video_id) = parsed["contents"]["twoColumnSearchResultsRenderer"]["primaryContents"]["sectionListRenderer"]["contents"]
                .get(0)
                .and_then(|content| content["itemSectionRenderer"]["contents"].get(0))
                .and_then(|item_content| item_content["videoRenderer"]["videoId"].as_str())
            {
                video_url = format!("https://www.youtube.com/watch?v={}", video_id);
                println!("{}", video_url);
                break;
            }
        }
    }

    return Ok(video_url);

}



