fn main() -> Result<(), Box<dyn ::std::error::Error>> {
    println!("cargo:rerun-if-changed=assets/index.css");

    let source = ::std::fs::read_to_string("assets/index.css")?;
    let mut stylesheet = ::lightningcss::stylesheet::StyleSheet::parse(
        &source,
        ::lightningcss::stylesheet::ParserOptions::default(),
    )
    .map_err(|e| e.to_string())?;
    stylesheet
        .minify(::lightningcss::stylesheet::MinifyOptions::default())
        .map_err(|e| e.to_string())?;
    let minified = stylesheet
        .to_css(::lightningcss::printer::PrinterOptions {
            minify: true,
            ..Default::default()
        })
        .map_err(|e| e.to_string())?;
    ::std::fs::write("assets/index.min.css", minified.code)?;

    Ok(())
}
