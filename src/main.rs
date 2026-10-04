mod desktop;
mod ui;

use mystline::{
    config::{self, InstanceLock},
    repository::TaskRepository,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dir = config::directory()?;
    let Some(_instance) = InstanceLock::acquire(&dir)? else {
        return Ok(());
    };
    let config = config::load_or_create(&dir)?;
    let repo = TaskRepository::open(config.tasks_file.clone(), true)?;
    if let Err(error) = desktop::set_autostart(&config) {
        eprintln!("Mystline : démarrage automatique non disponible : {error}");
    }
    ui::run(dir, config, repo)?;
    Ok(())
}
