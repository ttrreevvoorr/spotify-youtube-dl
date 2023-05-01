use reqwest::Client;
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, ACCEPT_ENCODING, ACCEPT_LANGUAGE, USER_AGENT};
use scraper::{Html, Selector};
use serde_json::Value;
use std::error::Error;
use tokio::main;
use std::fs::File;
use std::io::prelude::*;
use std::process::Command;
use rusty_ytdl::Video;
use rusty_ytdl::{choose_format, VideoOptions, VideoQuality, VideoSearchOptions };

struct Song {
    title: String,
    artist: String,
    album: String,
    url: String,
}

fn create_headers() -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(
        USER_AGENT,
        HeaderValue::from_static("Mozilla/5.0 (Macintosh; Intel Mac OS X x.y; rv:42.0) Gecko/20100101 Firefox/42.0")
    );
    headers.insert(
        ACCEPT,
        HeaderValue::from_static("text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.9")
    );
    headers.insert(
        ACCEPT_LANGUAGE,
        HeaderValue::from_static("en-US,en;q=0.9")
    );
    headers
}


#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let url = "https://open.spotify.com/playlist/10Wgl6LQiT5zr3hqRVmunQ?si=1cf93377485a4228";
    //let url = "https://open.spotify.com/album/6hPkbAV3ZXpGZBGUvL6jVM";
    let spotify_html = get_html_from_url(&url).await?;
    let page_tracks = get_tracks_from_html(&spotify_html);
    let binding = &page_tracks.unwrap();
    let track_data = get_track_data(binding).await;
    println!("{:?}", &track_data);
    download_videos(&track_data.unwrap()).await;

    Ok(())
}


// GET a URL and return the response as parsed HTML
async fn get_html_from_url(url: &str) -> Result<Html, Box<dyn Error>> {
    let client = Client::builder()
        .default_headers(create_headers())
        .build()?;
    let resp = client.get(url).send().await?;
    let html = resp.text().await.unwrap();
    let parsed_html = Html::parse_document(&html);

    return Ok(parsed_html);
}


fn get_tracks_from_html(parsed_html: &Html) -> Result<Vec<String>, Box<dyn Error>> {
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


// Get track information (title, artist, YouTube URL) for each track in the given list
async fn get_track_data(tracks: &Vec<String>) -> Result<Vec<String>, Box<dyn Error>> {
    let mut track_list = Vec::new();
    let selector = Selector::parse("meta").unwrap();

    for track in tracks {
        let html = get_html_from_url(&track).await?;
        let elements = html.select(&selector).collect::<Vec<_>>();

        let (mut title, mut artist) = ("".to_string(), "".to_string());

        for element in elements {
            if let (Some(content), Some(property)) = (element.value().attr("content"), element.value().attr("property")) {
                if property == "og:title" {
                    title = content.to_string();
                } else if property == "og:description" {
                    artist = content.split("·").next().unwrap().trim().to_string();
                }
            }
        }

        if !artist.is_empty() && !title.is_empty() {
            let query = format!("{} {} {}", title, artist, "audio");
            println!("Looking up \"{}\" on YouTube", query);
            let url = get_youtube_url(query).await?;
            track_list.push(url);
        }
    }
    Ok(track_list)
}

// Get and parse the HTML from youtube search results
async fn get_youtube_url(query: String) -> Result<String, Box<dyn Error>> {
    let url = format!("https://www.youtube.com/results?search_query={}", query);
    let html = reqwest::get(&url)
        .await
        .map_err(|err| format!("Error fetching YouTube results: {}", err))?
        .text()
        .await
        .map_err(|err| format!("Error reading YouTube response: {}", err))?;

    let parsed_html = Html::parse_document(&html);
    let selector = Selector::parse("script").unwrap();

    let variable_name = "ytInitialData";
    let video_url = parsed_html
        .select(&selector)
        .filter_map(|element| {
            let script_text = element.text().collect::<Vec<_>>().join("");
            if script_text.contains(&format!("var {} = ", variable_name)) {
                let start_index = script_text
                    .find('{')
                    .ok_or_else(|| format!("Error parsing JSON from script")).ok()?;

                let end_index = script_text
                    .rfind('}')
                    .ok_or_else(|| format!("Error parsing JSON from script")).ok()?;

                let json_str = &script_text[start_index..=end_index];
                let parsed: Value = serde_json::from_str(&json_str)
                    .map_err(|err| format!("Error parsing JSON from script: {}", err)).ok()?;

                if let Some(video_id) = parsed["contents"]["twoColumnSearchResultsRenderer"]["primaryContents"]["sectionListRenderer"]["contents"]
                    .get(0)
                    .and_then(|content| content["itemSectionRenderer"]["contents"].get(0))
                    .and_then(|item_content| item_content["videoRenderer"]["videoId"].as_str())
                {
                    println!("{:?}", format!("https://www.youtube.com/watch?v={}", video_id));
                    return Some(format!("https://www.youtube.com/watch?v={}", video_id));
                }
            }

            None
        })
        .next()
        .ok_or_else(|| format!("No video found for query: {}", query))?;

    Ok(video_url)
}

// 
async fn download_videos(urls: &Vec<String>) {
    for video_url in urls {
        // let video_url = "https://www.youtube.com/watch?v=FZ8BxMU3BYc"; // FZ8BxMU3BYc works too!
        let video = Video::new(video_url).unwrap();


        // Do what you want with video buffer vector
        let video_download_buffer = video.download().await;
        //dprintln!("{:#?}",video_download_buffer);

        // Or with options
        let video_options = VideoOptions {
          quality: VideoQuality::Lowest,
          filter: VideoSearchOptions::Audio,
          ..Default::default()
        };


        let video_info = video.get_info().await.unwrap();
        let format = choose_format(&video_info.formats, &video_options);
        println!("{:?}", format);


        let video = Video::new_with_options(video_url, video_options).unwrap();
        let video_download_buffer = video.download().await.unwrap();

        // Create a new file and open it for writing
        let mut file = match File::create(format!("{}.mp3", video_url)) {
            Ok(file) => file,
            Err(error) => {
                println!("Error creating file: {:?}", error);
                return;
            }
        };

        // Write the video buffer to the file
        match file.write_all(&video_download_buffer) {
            Ok(()) => println!("File written successfully"),
            Err(error) => println!("Error writing file: {:?}", error),
        };
    }
}

