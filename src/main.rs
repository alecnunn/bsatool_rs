extern crate clap;
extern crate indicatif;
use clap::{App, Arg, SubCommand};
use indicatif::ProgressBar;

use std::fs;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

mod bsa;

struct Arguments {
    mode: String,
    filename: String,
    extractfile: String,
    outdir: String,

    longformat: bool,
    fullpath: bool,
}

impl Arguments {
    pub fn new() -> Self {
        Self {
            mode: String::from("list"),
            filename: String::from(""),
            extractfile: String::from(""),
            outdir: String::from("."),
            longformat: false,
            fullpath: false,
        }
    }
}

fn parse_options() -> Arguments {
    let mut info = Arguments::new();

    let matches = App::new("bsatool")
        .version("0.1.0")
        .author("arviceblot <logan@arviceblot.com>")
        .about("Inspect and extract files from Bethesda BSA archives")
        .arg(
            Arg::with_name("INPUT")
                .required(true)
                .help("The input archive file to use"),
        )
        .subcommand(
            SubCommand::with_name("list")
                .about("List the files presents in the input archive")
                .arg(
                    Arg::with_name("long")
                        .short("l")
                        .long("long")
                        .help("Include extra information in archive listing"),
                ),
        )
        .subcommand(
            SubCommand::with_name("extract")
                .about("Extract a file from the input archive")
                .arg(Arg::with_name("full-path")
                    .short("f")
                    .long("full-path")
                    .help("Create directory hierarchy on file extraction (always true for extractall)",
                ))
                .arg(Arg::with_name("file_to_extract"))
                .arg(Arg::with_name("output_directory")),
        )
        .subcommand(
            SubCommand::with_name("extractall")
                .about("Extract all files from the input archive")
                .arg(Arg::with_name("output_directory")),
        )
        .get_matches();

    info.filename = matches.value_of("INPUT").unwrap().to_string();

    if let Some(matches) = matches.subcommand_matches("list") {
        info.mode = String::from("list");
        info.longformat = matches.is_present("long");
    } else if let Some(matches) = matches.subcommand_matches("extract") {
        info.mode = String::from("extract");
        info.fullpath = matches.is_present("full-path");
        info.extractfile = matches
            .value_of("file_to_extract")
            .unwrap_or("")
            .to_string();
        info.outdir = matches
            .value_of("output_directory")
            .unwrap_or(".")
            .to_string();
    } else if let Some(matches) = matches.subcommand_matches("extractall") {
        info.mode = String::from("extractall");
        info.outdir = matches
            .value_of("output_directory")
            .unwrap_or(".")
            .to_string();
    }
    info
}

fn list(bsa: &bsa::BSAFile, info: &Arguments) {
    let files = bsa.get_list();
    for file in files {
        if info.longformat {
            println!("{:50}{:8}@ 0x{:x}", file.name, file.file_size, file.offset)
        } else {
            println!("{}", file.name)
        }
    }
}

fn extract(bsa: &bsa::BSAFile, info: &Arguments) {
    let archive_path = &info.extractfile.to_string().replace("/", "\\");
    let extract_path = &info.extractfile.to_string().replace("\\", "/");

    if !bsa.exists(archive_path.to_string()) {
        panic!(
            "ERROR: file '{}' not found
In archive: {}",
            info.extractfile.to_string(),
            info.filename.to_string()
        )
    }

    let rel_path = Path::new(&extract_path);
    let mut target = PathBuf::from(&info.outdir);
    if info.fullpath {
        target.push(rel_path);
    } else {
        target.push(rel_path.file_name().unwrap());
    }

    // Create the directory hierarchy
    fs::create_dir_all(target.parent().unwrap()).unwrap();

    if !target.parent().unwrap().is_dir() {
        panic!(
            "ERROR: {} is not a directory.",
            target.parent().unwrap().to_str().unwrap()
        );
    }

    // Get a buffer for the file to extract
    // (inefficient because get_file iter on the list again)
    let data = bsa.get_file(archive_path.to_string());

    // Write the file to disk
    println!(
        "Extracting {} to {}",
        info.extractfile.to_string(),
        target.to_str().unwrap().to_string()
    );
    let f = File::create(target).expect("Unable to create file");
    let mut f = BufWriter::new(f);
    f.write_all(&data).unwrap();
    f.flush().unwrap();
}

fn extractall(bsa: &bsa::BSAFile, info: &Arguments) {
    // Get the list of files present in the archive
    let list = bsa.get_list();
    let pb = ProgressBar::new(list.len() as u64);

    for file in list {
        pb.inc(1);
        let extract_path = file.name.to_string().replace("\\", "/");

        // Get the target path (the path the file will be extracted to)
        let target = Path::new(&info.outdir).join(extract_path.to_string());

        // Create the directory hierarchy
        fs::create_dir_all(target.parent().unwrap()).unwrap();

        if !target.parent().unwrap().is_dir() {
            panic!(
                "ERROR: {} is not a directory.",
                target.parent().unwrap().to_str().unwrap()
            );
        }

        // Get a buffer for the file to extract
        // (inefficient because get_file iter on the list again)
        let data = bsa.get_file(file.name.to_string());

        // Write the file to disk
        let f = File::create(target).expect("Unable to create file");
        let mut f = BufWriter::new(f);
        f.write_all(&data).unwrap();
        f.flush().unwrap();
    }
    pb.finish_with_message("done");
}

fn main() {
    let info = parse_options();

    // Open file
    let mut bsa: bsa::BSAFile = bsa::BSAFile::new();
    bsa.open(info.filename.to_string());

    match info.mode.as_str() {
        "list" => list(&bsa, &info),
        "extract" => extract(&bsa, &info),
        "extractall" => extractall(&bsa, &info),
        _ => println!("Unsupported mode. That is not supposed to happen."),
    }
}
