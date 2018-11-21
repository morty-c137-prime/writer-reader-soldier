extern crate printpdf;
extern crate clap;
extern crate glob;

use clap::{Arg, App};
use printpdf::*;
use glob::{glob, Paths};

use std::env;
use std::path::{PathBuf};
use std::fs::File;
use std::process;
use std::io::prelude::*;
use std::io::{self, BufReader, BufWriter};

fn read_file(file_location: &PathBuf) -> Vec<String>
{
    let file = File::open(file_location).unwrap_or_else(|err| {
        println!("RWS could not find files supplied... {:?}", err);
        process::exit(1);
    });
    let reader = BufReader::new(&file);

    let mut lines = Vec::new();
    for (_index, line) in reader.lines().enumerate()
    {
        lines.push(line.unwrap());
    }
    lines
}

enum FormatType
{
    Title,
    Newline,
    Text
}

struct Instruction
{
    data: String,
    format: FormatType,
}

const HEIGHT: Mm = Mm(279.4);
const WIDTH: Mm = Mm(215.9);

fn main() {
    let matches = App::new("Writer Reader Soldier")
        .version("1.0")
        .author("Richard Alvarez <rawalvarez731@gmail.com>")
        .about("Formatted files into a pretty pdfs")
        .arg(Arg::with_name("debug_mode")
            .short("d"))
        .arg(Arg::with_name("files")
            .short("f")
            .long("files")
            .long_help("followed by any number of glob patterns to select files for formatting")
            .required(false)
            .takes_value(true)
            .multiple(true))
        .arg(Arg::with_name("output")
            .short("o")
            .required(true)
            .takes_value(true))
        .arg(Arg::with_name("title")
            .short("t")
            .long("title")
            .required(true)
            .takes_value(true)
            .long_help("the title of the PDF document that wil be generated"))
        .get_matches();

    // ? File names and file locations
    let globs = matches.values_of("files").unwrap();
    let file_locations = globs.map(|globstr| {
        glob(globstr)
            .expect("Failed to glob...")
    })
    .flat_map(|files: Paths| files.flatten())
    .map(|incomplete_file_location| {
        let mut correct_ending = incomplete_file_location;
        if correct_ending.is_dir() { correct_ending.push("/") }

        let mut complete_location = env::current_dir().unwrap();
        complete_location.push(correct_ending);
        complete_location
    });

    let mut build_instructions: Vec<Vec<Instruction>> = Vec::new();

    for file_location in file_locations
    {
        let lines: Vec<String> = read_file(&file_location);
        let mut instructions: Vec<Instruction> = Vec::new();

        for line in lines.iter()
        {
            if line.contains("// unfinished") || line.contains("//unfinished") {
                break;
            }
            if line == "" || line == " " || line == "\n" {
                instructions.push(Instruction {
                    data: line.to_owned(),
                    format: FormatType::Newline,
                });
                continue;
            }
            if line.starts_with("#") || line.starts_with(" #") {
                let title = line.split("#")
                    .last()
                    .unwrap_or("Untitled");

                instructions.push(Instruction {
                    data: title.trim_left().to_owned(),
                    format: FormatType::Title,
                });
                continue;
            }
            instructions.push(Instruction {
                data: line.to_owned(),
                format: FormatType::Text,
            });
        }
        build_instructions.push(instructions);
    }

    // * This is the ascii output... now lets do the PDF output.
    for build_instruction in build_instructions.iter()  {
        'builds: for instruction in build_instruction
        {
            match instruction.format
            {
                FormatType::Title => {
                    println!("{}", instruction.data);
                    for _i in 0..instruction.data.len() + 1
                    {
                        print!("-");
                    }
                    println!("");
                },
                FormatType::Newline => {
                    println!("");
                },
                FormatType::Text => {
                    println!("{}", instruction.data);
                }
            }
        }
        println!("\n~\n");
    }

    let (doc, page1, layer1) = PdfDocument::new(matches.value_of("title").unwrap_or("Untitled"), WIDTH, HEIGHT,  "Page 1, Layer 1");
    let font = doc.add_external_font(File::open("assets/fonts/Calibri.ttf").unwrap()).unwrap();

    let mut pages: Vec<_> = Vec::new();
    pages.push(doc.get_page(page1).get_layer(layer1));
    let mut wrote: bool = false;

    for (build_index, build_instruction) in build_instructions.iter().enumerate()  {
        if build_instruction.iter().count() == 0 { continue }

        if build_index != 0 && wrote {
            let (npage, nlayer) = doc.add_page(WIDTH, HEIGHT, format!("Page {}, Layer 1", build_index));
            pages.push(doc.get_page(npage).get_layer(nlayer));
        }

        let mut text_elements = 0.0;
        let current_layer = pages.last().unwrap();
        'builds: for instruction in build_instruction.iter()
        {
            current_layer.begin_text_section();
                match instruction.format
                {
                    FormatType::Title => {
                        current_layer.set_font(&font, 24);
                        current_layer.set_text_cursor(Mm(10.0), Mm(260.0));
                        current_layer.set_line_height(8 );
                        current_layer.set_word_spacing(3);
                        current_layer.set_character_spacing(2);
                        current_layer.set_text_rendering_mode(TextRenderingMode::FillStroke);

                        // write two lines (one line break)
                        current_layer.write_text(instruction.data.clone(), &font);
                        current_layer.add_line_break();
                        wrote = true;
                    },
                    _ => {
                        text_elements += 1.0;
                        current_layer.set_font(&font, 11);
                        current_layer.set_text_cursor(Mm(10.0), Mm(256.0 - (text_elements * 3.86) - 2.0));
                        current_layer.set_line_height(3);
                        current_layer.set_word_spacing(3);
                        current_layer.set_character_spacing(2);
                        current_layer.set_text_rendering_mode(TextRenderingMode::FillStroke);

                        // write two lines (one line break)
                        current_layer.write_text(instruction.data.clone(), &font);
                        current_layer.add_line_break();
                        wrote = true;
                    }
                }
            current_layer.end_text_section();
        }
    }
    doc.save(&mut BufWriter::new(File::create(matches.value_of("output").unwrap()).unwrap())).unwrap();
}
