use anyhow::{Context, Result};
use jcode_experiment_flags::{
    Experiments, Stage, EXPERIMENT_FLAGS,
};

pub fn run_experiment_list_command(json: bool) -> Result<()> {
    let config = crate::config::config();
    let experiments = Experiments::from_config(&config.experiments.entries);

    if json {
        let states = experiments.all_flag_states();
        println!("{}", serde_json::to_string_pretty(&states)?);
    } else {
        println!(
            "{:25} {:25} {:8} {:8}  {}",
            "Key", "Flag", "Default", "Current", "Stage"
        );
        println!("{}", "-".repeat(90));
        for spec in EXPERIMENT_FLAGS {
            let enabled = experiments.check(spec.id);
            let default_str = if spec.default_enabled { "on" } else { "off" };
            let current_str = if enabled { "ON" } else { "OFF" };
            let stage_label = match spec.stage {
                Stage::UnderDevelopment => "UnderDevelopment",
                Stage::Experimental { .. } => "Experimental",
                Stage::Stable => "Stable",
                Stage::Deprecated { .. } => "Deprecated",
                Stage::Removed => "Removed",
            };
            println!(
                "{:25} {:25} {:8} {:8}  {}",
                spec.key,
                format!("{:?}", spec.id),
                default_str,
                current_str,
                stage_label,
            );
        }
    }
    Ok(())
}

pub fn run_experiment_enable_command(key: &str) -> Result<()> {
    let mut config = crate::config::Config::load();
    config.experiments.entries.insert(key.to_string(), true);
    config.save().context("Failed to save config")?;
    crate::config::invalidate_config_cache();
    eprintln!("[jcode] Experiment '{key}' enabled.");
    Ok(())
}

pub fn run_experiment_disable_command(key: &str) -> Result<()> {
    let mut config = crate::config::Config::load();
    config.experiments.entries.insert(key.to_string(), false);
    config.save().context("Failed to save config")?;
    crate::config::invalidate_config_cache();
    eprintln!("[jcode] Experiment '{key}' disabled.");
    Ok(())
}
