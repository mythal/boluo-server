use fern::colors::{Color, ColoredLevelConfig};
use log::LevelFilter;

pub fn setup_logger(debug: bool) -> Result<(), fern::InitError> {
    let level = if debug { LevelFilter::Debug } else { LevelFilter::Info };
    let color_config = ColoredLevelConfig::new()
        .info(Color::BrightGreen)
        .error(Color::BrightRed)
        .warn(Color::Yellow)
        .debug(Color::Magenta)
        .trace(Color::BrightCyan);

    fern::Dispatch::new()
        .format(move |out, message, record| {
            out.finish(format_args!(
                "[{:>5}]{}[{}] {}",
                color_config.color(record.level()),
                chrono::Local::now().format("[%Y-%m-%d][%H:%M:%S]"),
                record.target(),
                message
            ))
        })
        .level(LevelFilter::Info)
        .level_for("server", level)
        .chain(std::io::stdout())
        .chain(fern::log_file("server.log")?)
        .apply()?;
    Ok(())
}
