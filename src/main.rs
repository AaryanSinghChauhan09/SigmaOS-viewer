//! SigmaOS-viewer — image viewer CLI.
//! Usage: viewer <dir|file> [next|prev|first|last|info|stats|formats|tag K V|rate N]

use viewer::{HostFs, ImageViewer};
use std::path::PathBuf;
use std::process::exit;

fn usage() -> ! {
    eprintln!("usage: viewer <path> [cmd]");
    exit(2);
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() { usage(); }
    let path = PathBuf::from(&args[0]);
    let cmd = args.get(1).map(|s| s.as_str()).unwrap_or("info");
    let mut v = ImageViewer::new(HostFs);

    // if path is a file, load its parent dir and jump to it
    let target = if path.is_dir() {
        path.clone()
    } else {
        path.parent().map(|p| p.to_path_buf()).unwrap_or_else(|| PathBuf::from("."))
    };
    let n = match v.load_dir(&target) {
        Ok(n) => n,
        Err(e) => { eprintln!("viewer: {}", e); exit(1); }
    };
    if !path.is_dir() {
        if let Some(pos) = v.gallery.iter().position(|i| i.path == path) {
            let _ = v.go_to(pos);
        }
    }

    let r: Result<(), String> = (|| {
        match cmd {
            "info" => {
                println!("{} images in {}", n, target.display());
                if let Some(card) = v.info_card() { println!("{}", card); }
            }
            "next" => { v.next().map_err(|e| e.to_string())?;
                if let Some(c) = v.info_card() { println!("{}", c); } }
            "prev" => { v.prev().map_err(|e| e.to_string())?;
                if let Some(c) = v.info_card() { println!("{}", c); } }
            "first" => { v.first(); if let Some(c)=v.info_card(){println!("{}",c);} }
            "last" => { v.last(); if let Some(c)=v.info_card(){println!("{}",c);} }
            "stats" => {
                let s = v.stats();
                println!("{} images, {} total, {} animated, {} annotated",
                    s.count, viewer::human_bytes(s.total_bytes), s.animated, s.annotated);
                for (f, c) in &s.by_format { println!("  {:<8} {}", f, c); }
            }
            "formats" => {
                for f in viewer::ImageFormat::sniff(b"\x89PNG") .into_iter() {}
                // list supported
                println!("png jpeg gif bmp webp tiff svg ico pnm qoi");
            }
            "tag" => {
                if args.len() < 4 { return Err("tag <rating> <tag...>".into()); }
                let rating: u8 = args[2].parse().map_err(|_| "bad rating")?;
                let tags: Vec<String> = args[3..].to_vec();
                let cap = tags.join(" ");
                v.annotate(rating, tags.clone(), cap);
                println!("annotated");
            }
            "rate" => {
                if args.len() < 3 { return Err("rate <0-5>".into()); }
                let r: u8 = args[2].parse().map_err(|_| "bad rating")?;
                v.annotate(r, vec![], "");
                println!("rated {}/5", r.min(5));
            }
            _ => usage(),
        }
        Ok(())
    })();
    if let Err(e) = r { eprintln!("viewer: {}", e); exit(1); }
}
