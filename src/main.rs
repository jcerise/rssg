use std::error::Error;
use serde_derive::{Deserialize, Serialize};
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use config::Config;
use pulldown_cmark::{html, Options, Parser};
use tera::{Context, Tera};
use yaml_front_matter::YamlFrontMatter;

#[derive(Serialize, Deserialize)]
struct PageInfo {
    title: String,
    description: String,
    tags: Vec<String>,
    similar_posts: Vec<String>,
    date: String,
    favorite_numbers: Vec<f64>,
    path: String,
}

#[derive(Debug, Deserialize)]
struct SiteConfig {
    site_title: String,
    base_url: String,
    theme: String,
    content_location: String,
    output_location: String,
}

fn load_config() -> Result<SiteConfig, Box<dyn Error>> {
    let settings = Config::builder()
        .add_source(config::File::with_name("./config"))
        .build()?;
    settings.try_deserialize::<SiteConfig>().map_err(|e| e.into())
}

fn read_dir(path: &str) -> std::io::Result<Vec<std::path::PathBuf>> {
    let mut files = Vec::new();
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            files.push(path);
        }
    }
    Ok(files)
}

fn parse_markdown(content: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_YAML_STYLE_METADATA_BLOCKS);
    let parser = Parser::new_ext(content, options);
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);
    html_output
}

fn render_template(tera: &Tera, html_content: &str, title: &String, config: &SiteConfig) -> Result<String, tera::Error> {
    let mut context = Context::new();
    context.insert("post_title", title);
    context.insert("site_title", &config.site_title);
    context.insert("content", html_content);
    context.insert("base_url", &config.base_url);

    tera.render("template.html", &context)
}

fn write_output_file(output_path: &Path, content: &str) -> std::io::Result<()> {
    let mut file = File::create(output_path)?;
    file.write_all(content.as_bytes())
}

fn generate_site(input_dir: &str, output_dir: &str, tera: &Tera, config: SiteConfig) -> std::io::Result<()>{
    fs::create_dir_all(output_dir)?;

    let files = read_dir(input_dir)?;
    let mut pages: Vec<PageInfo> = Vec::new();

    for file_path in files {
        let content = fs::read_to_string(&file_path)?;
        let mut page_info = collect_metadata(&content)?;
        let html_content = parse_markdown(&content);
        let rendered_content = render_template(tera, &html_content, &page_info.title, &config).unwrap();
        let output_file_path =
            Path::new(output_dir)
                .join(
                    Path::new(
                        Path::new(&file_path).file_stem().unwrap().to_str().unwrap()
                    ).with_extension("html")
                );

        write_output_file(&output_file_path, &rendered_content)?;
        page_info.path = output_file_path.file_name().unwrap().to_str().unwrap().to_string();
        pages.push(page_info);
    }

    generate_home_page(&output_dir, tera, &pages, &config)?;

    Ok(())
}

fn generate_home_page(output_dir: &str, tera: &Tera, pages: &[PageInfo], config: &SiteConfig) -> std::io::Result<()> {
    let mut context = Context::new();
    context.insert("site_title", &config.site_title);
    context.insert("pages", pages);
    context.insert("base_url", &config.base_url);


    let rendered_home = tera.render("index.html", &context).unwrap();
    let output_file_path = Path::new(output_dir).join("index.html");
    write_output_file(&output_file_path, &rendered_home)
}

fn collect_metadata(content: &str) -> std::io::Result<PageInfo> {
    let result = YamlFrontMatter::parse::<PageInfo>(&content);

    match result {
        Ok(data) => {
            let page_info = match data.metadata {
                PageInfo {title, description, tags, similar_posts, date, favorite_numbers, path} => {
                    PageInfo {
                        title,
                        description,
                        tags,
                        similar_posts,
                        date,
                        favorite_numbers,
                        path: "".to_string(),
                    }
                },
            };
            Ok(page_info)
        },
        Err(e) => Err(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())),
    }
}

fn main() -> Result<(), Box<dyn Error>>{
    // Initialize Tera for template rendering
    let tera = match Tera::new("templates/**/*") {
        Ok(t) => t,
        Err(e) => {
            println!("Parsing error(s): {}", e);
            ::std::process::exit(1);
        }
    };

    let config = load_config()?;

    let input_dir = format!("./{}", &config.content_location);
    let output_dir = format!("./{}", &config.output_location);

    generate_site(&input_dir, &output_dir, &tera, config)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use pulldown_cmark::Event;
    use pulldown_cmark::Tag;

    #[test]
    fn test_read_dir() {
        let temp_dir = tempdir::TempDir::new("rustrover_test").unwrap();
        let temp_path = temp_dir.path().to_str().unwrap();

        let file_path = format!("{}/test_file.txt", temp_path);
        fs::File::create(&file_path).unwrap();

        read_dir(temp_path).expect("Could not read directory");
    }

    #[test]
    #[should_panic(expected = "The system cannot find the path specified.")]
    fn test_read_dir_invalid_path() {
        read_dir("invalid_dir").unwrap();
    }

    #[test]
    fn test_parse_markdown_header() {
        let markdown_content = "# Hello, World!";
        let html_output = parse_markdown(markdown_content);

        assert_eq!(html_output, "<h1>Hello, World!</h1>\n");
    }

    #[test]
    fn test_parse_markdown_paragraph() {
        let markdown_content = "This is a paragraph.";
        let html_output = parse_markdown(markdown_content);

        assert_eq!(html_output, "<p>This is a paragraph.</p>\n");
    }

    #[test]
    fn test_parse_markdown_link() {
        let markdown_content = "[Rust website](https://www.rust-lang.org/)";
        let html_output = parse_markdown(markdown_content);

        assert_eq!(html_output, "<p><a href=\"https://www.rust-lang.org/\">Rust website</a></p>\n");
    }
}
