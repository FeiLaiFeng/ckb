mod helper;
mod subcommand;

use ckb_app_config::{cli, ExitCode, Setup};
use ckb_build_info::Version;

pub(crate) const LOG_TARGET_MAIN: &str = "main";

pub fn run_app(version: Version) -> Result<(), ExitCode> {
    // Always print backtrace on panic.
    ::std::env::set_var("RUST_BACKTRACE", "full");

    let app_matches = cli::get_matches(&version);
    match app_matches.subcommand() {
        (cli::CMD_INIT, Some(matches)) => {
            return subcommand::init(Setup::init(&matches)?);
        }
        (cli::CMD_LIST_HASHES, Some(matches)) => {
            return subcommand::list_hashes(Setup::root_dir_from_matches(&matches)?, matches);
        }
        _ => {
            // continue
        }
    }

    let setup = Setup::from_matches(&app_matches)?;
    let _guard = setup.setup_app(&version);

    match app_matches.subcommand() {
        (cli::CMD_RUN, Some(matches)) => subcommand::run(setup.run(&matches)?, version),
        (cli::CMD_MINER, _) => subcommand::miner(setup.miner()?),
        (cli::CMD_PROF, Some(matches)) => subcommand::profile(setup.prof(&matches)?),
        (cli::CMD_EXPORT, Some(matches)) => subcommand::export(setup.export(&matches)?),
        (cli::CMD_IMPORT, Some(matches)) => subcommand::import(setup.import(&matches)?),
        (cli::CMD_STATS, Some(matches)) => subcommand::stats(setup.stats(&matches)?),
        (cli::CMD_RESET_DATA, Some(matches)) => subcommand::reset_data(setup.reset_data(&matches)?),
        _ => unreachable!(),
    }
}
