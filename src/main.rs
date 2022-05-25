use clap::{Parser};
use pbr::{ProgressBar, Units};
use std::time::Duration;
use std::{path::{Path}, thread};
use regex::{Regex};
use tokio::{join, fs, io::AsyncWriteExt};

const USER_AGNET: &'static str = "Mozilla/5.0 (iPhone; CPU iPhone OS 13_2_3 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/13.0.3 Mobile/15E148 Safari/604.1";

/// Douyin video downloader.
#[derive(Parser, Debug)]
#[clap(version, author, about)]
struct Args {
    /// Douyin video sharing link.
    #[clap(short, long)]
    link: String,

    /// Download video, cover and music.
    #[clap(short, long)]
    all: bool,

    /// Download music.
    #[clap(short, long)]
    music: bool,

    /// Download cover.
    #[clap(short, long)]
    cover: bool,
    
    /// Download music.
    #[clap(short, long)]
    video: bool,
    
    /// File storage directory, default: ./download
    #[clap(short, long, default_value_t=String::from("download"))]
    dir: String,
}

struct Link {
    aweme_id: String,
    video_url: String,
    cover_url: String,
    music_url: String,
}

enum DownloadType {
    Music,
    Cover,
    Video,
}

/// 解析视频,音频,封面URL
async fn parse(link: String) -> Result<Link, Box<dyn std::error::Error>> {

    let resp = reqwest::get(link).await?;
    let remote_url = resp.url().as_str().to_string();
    let reg = Regex::new(r"/(\d+)\?")?;
    let mut aweme_id  = String::new();

    if let Some(m) = reg.find(&remote_url) {
        let tmp = &mut aweme_id ;
        *tmp = m.as_str().to_string().replace("/", "").replace("?", "");
    }

    let api_url = "https://www.iesdouyin.com/web/api/v2/aweme/iteminfo/?item_ids=".to_string() + &aweme_id;
    let res_data = reqwest::get(api_url).await?.text().await?;
    
    let json_data = json::parse(&res_data).unwrap();
    let cover_url:String = json_data["item_list"][0]["video"]["origin_cover"]["url_list"][0].clone().to_string();
    let music_url = json_data["item_list"][0]["music"]["play_url"]["url_list"][0].clone().to_string();
    let video_url = json_data["item_list"][0]["video"]["play_addr"]["url_list"][0].clone().to_string().replace("playwm", "playwm").replace("ratio=720p", "ratio=1080p");
    Ok(Link {aweme_id, video_url, cover_url, music_url})
    
}

/// 文件下载
#[warn(unreachable_patterns)]
async fn download(aweme_id:String, link: String, mode: DownloadType, dirname: String) -> Result<(), Box<dyn std::error::Error>> {
    
    if !Path::new(&dirname).exists(){
        fs::create_dir(&dirname).await?;
    }

    let filename:String = match mode {
        DownloadType::Cover => format!("{}/cover_{}.jpeg", dirname, aweme_id),
        DownloadType::Music => format!("{}/music_{}.m4a", dirname, aweme_id),
        DownloadType::Video => format!("{}/video_{}.mp4", dirname, aweme_id),
        _ => "".to_string(),
    };

    let client = reqwest::Client::builder()
    .user_agent(USER_AGNET)
    .build()?;

    let respone = client.get(&link).send().await?;
    let content_length = respone.content_length().unwrap(); 
    
    let mut pb = ProgressBar::new(content_length);
    pb.set_units(Units::Bytes);

    let mut source = client.get(&link).send().await?;
 
    let mut dest = fs::OpenOptions::new().create(true).append(true).open(&filename).await?;
    
    while let Some(chunk) = source.chunk().await? {
        dest.write_all(&chunk).await?;
        pb.add(chunk.len() as u64);
        thread::sleep(Duration::from_millis(20));
    }
    pb.finish_println(format!("{}, 下载成功!\n", filename).as_str());
    Ok(())
}

/// 功能入口
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>>{
    let mut  args = Args::parse();

    if args.all { // download all
        args.music = true;
        args.cover = true;
        args.video = true;
    }


    if let Ok(link) = parse(args.link).await {
        if args.cover {
            let downloader = download(link.aweme_id.clone(), link.cover_url, DownloadType::Cover, args.dir.clone());
            let _ = join!(downloader);
        }
        if args.music {
            let music_dl = download(link.aweme_id.clone(), link.music_url, DownloadType::Music, args.dir.clone());
            let _ = join!(music_dl);
        }
        if args.video {
            let video_dl = download(link.aweme_id.clone(), link.video_url, DownloadType::Video, args.dir.clone());
            let _ = join!(video_dl);
        }
        
        
    }else {
        println!("下载失败, 无法解析地址...");
    }

    Ok(())
}