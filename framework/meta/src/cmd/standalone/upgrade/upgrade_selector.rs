use crate::{
    cli_args::UpgradeArgs,
    cmd::standalone::upgrade::upgrade_settings::UpgradeSettings,
    folder_structure::{dir_pretty_print, RelevantDirectories, RelevantDirectory},
    version_history::{versions_iter, LAST_UPGRADE_VERSION, MILESTONE_VERSION, VERSIONS},
};

use super::{
    upgrade_0_31::upgrade_to_31_0,
    upgrade_0_32::upgrade_to_32_0,
    upgrade_0_39::{postprocessing_after_39_0, upgrade_to_39_0},
    upgrade_0_45::upgrade_to_45_0,
    upgrade_common::{cargo_check, version_bump_in_cargo_toml},
    upgrade_print::*,
};

pub fn upgrade_sc(args: &UpgradeArgs) {
    let path = if let Some(some_path) = &args.path {
        some_path.as_str()
    } else {
        "./"
    };

    let settings = UpgradeSettings::new(args.no_check);

    let last_version = args
        .override_target_version
        .clone()
        .map(|override_target_v| {
            VERSIONS
                .iter()
                .find(|&v| v.version.to_string() == override_target_v)
                .unwrap_or_else(|| &LAST_UPGRADE_VERSION)
        })
        .unwrap_or_else(|| &LAST_UPGRADE_VERSION);    

    assert!(
        VERSIONS.contains(&last_version),
        "Invalid requested version: {}",
        last_version.version.to_string(),
    );

    let mut dirs = RelevantDirectories::find_all(path, args.ignore.as_slice());
    println!(
        "Found {} directories to upgrade, out of which {} are contract crates.\n",
        dirs.len(),
        dirs.iter_contract_crates().count(),
    );
    dir_pretty_print(dirs.iter(), "", &|dir| {
        print_tree_dir_metadata(dir, last_version)
    });

    for (from_version, to_version) in versions_iter(last_version.clone()) {
        if dirs.count_for_version(from_version) == 0 {
            continue;
        }

        print_upgrading_all(
            from_version.version.to_string().as_str(),
            &to_version.version.to_string().as_str(),
        );
        dirs.start_upgrade(from_version, to_version);
        for dir in dirs.iter_version(from_version) {
            upgrade_function_selector(dir);
        }

        for dir in dirs.iter_version(from_version) {
            upgrade_post_processing(dir, &settings);
        }

        // // change the version in memory for the next iteration (dirs is not reloaded from disk)
        // dirs.update_versions_in_memory(from_version, to_version);
        dirs.finish_upgrade();
    }
}

fn upgrade_function_selector(dir: &RelevantDirectory) {
    if dir.upgrade_in_progress.is_some() {
        print_upgrading(dir);
    }

    match dir.upgrade_in_progress {
        Some((from_version, to_version)) => match to_version.version.to_string().as_str() {
            "0.31.0" => upgrade_to_31_0(dir),
            "0.32.0" => upgrade_to_32_0(dir),
            "0.39.0" => upgrade_to_39_0(dir),
            "0.45.0" => upgrade_to_45_0(dir),
            _ => version_bump_in_cargo_toml(&dir.path, from_version, to_version),
        },
        None => {},
    }
}

fn upgrade_post_processing(dir: &RelevantDirectory, settings: &UpgradeSettings) {
    match dir.upgrade_in_progress {
        Some((_, to_version))
            if [
                "0.28.0", "0.29.0", "0.30.0", "0.31.0", "0.32.0", "0.33.0", "0.34.0", "0.35.0",
                "0.36.0", "0.37.0", "0.40.0", "0.41.0", "0.42.0", "0.43.0", "0.44.0", "0.45.2",
            ]
            .contains(&to_version.version.to_string().as_str()) =>
        {
            print_post_processing(dir);
            cargo_check(dir, settings);
        },
        Some((_, MILESTONE_VERSION)) => {
            print_post_processing(dir);
            postprocessing_after_39_0(dir);
            cargo_check(dir, settings);
        },
        _ => {},
    }
}
