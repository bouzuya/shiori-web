use lightningcss::stylesheet::MinifyOptions;
use lightningcss::stylesheet::ParserOptions;
use lightningcss::stylesheet::PrinterOptions;
use lightningcss::stylesheet::StyleSheet;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=assets/index.css");

    let source = std::fs::read_to_string("assets/index.css")?;
    let mut stylesheet =
        StyleSheet::parse(&source, ParserOptions::default()).map_err(|e| e.to_string())?;
    stylesheet
        .minify(MinifyOptions::default())
        .map_err(|e| e.to_string())?;
    let minified = stylesheet
        .to_css(PrinterOptions {
            minify: true,
            ..Default::default()
        })
        .map_err(|e| e.to_string())?;
    std::fs::write("assets/index.min.css", minified.code)?;

    Ok(())
}
