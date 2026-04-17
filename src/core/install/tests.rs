use std::fs;
use std::path::Path;

use tempfile::tempdir;

use super::{
    HealthStatus, HostPlatform, WowFlavor, discover_local_accounts, inspect_installation_on_host,
};

#[test]
fn inspect_installation_resolves_product_root() {
    let temp = tempdir().expect("temp dir");
    let product_root = temp.path().join("World of Warcraft");
    let flavor_root = product_root.join("_retail_");

    fs::create_dir_all(flavor_root.join("Interface").join("AddOns")).expect("addons dir");
    fs::create_dir_all(flavor_root.join("WTF")).expect("wtf dir");
    fs::write(
        flavor_root.join("WTF").join("Config.wtf"),
        "SET locale enUS",
    )
    .expect("config");

    let inspection = inspect_installation_on_host(
        &product_root,
        Some(WowFlavor::Retail),
        HostPlatform::current(),
    )
    .expect("inspect");

    assert_eq!(inspection.installation.flavor, WowFlavor::Retail);
    assert_eq!(inspection.health.status, HealthStatus::Warning);
    assert!(
        inspection
            .installation
            .flavor_root
            .ends_with(Path::new("World of Warcraft").join("_retail_"))
    );
}

#[test]
fn discover_local_accounts_reads_accounts_and_characters() {
    let temp = tempdir().expect("temp dir");
    let product_root = temp.path().join("World of Warcraft");
    let flavor_root = product_root.join("_retail_");

    fs::create_dir_all(flavor_root.join("Interface").join("AddOns")).expect("addons dir");
    fs::create_dir_all(
        flavor_root
            .join("WTF")
            .join("Account")
            .join("SavedVariables"),
    )
    .expect("global saved variables");
    fs::create_dir_all(
        flavor_root
            .join("WTF")
            .join("Account")
            .join("ACC1")
            .join("SavedVariables"),
    )
    .expect("saved variables");
    fs::create_dir_all(
        flavor_root
            .join("WTF")
            .join("Account")
            .join("ACC1")
            .join("Illidan")
            .join("Mageone")
            .join("SavedVariables"),
    )
    .expect("character");
    fs::write(
        flavor_root.join("WTF").join("Config.wtf"),
        "SET locale enUS",
    )
    .expect("config");

    let installation = inspect_installation_on_host(
        &product_root,
        Some(WowFlavor::Retail),
        HostPlatform::current(),
    )
    .expect("inspect");
    let accounts = discover_local_accounts(&installation.installation).expect("discover accounts");

    assert_eq!(accounts.len(), 1);
    assert_eq!(accounts[0].account_name, "ACC1");
    assert!(
        accounts[0]
            .saved_variables_dir
            .ends_with(Path::new("ACC1").join("SavedVariables"))
    );
    assert_eq!(accounts[0].characters.len(), 1);
    assert_eq!(accounts[0].characters[0].server, "Illidan");
    assert_eq!(accounts[0].characters[0].character, "Mageone");
}

#[test]
fn discover_local_accounts_ignores_noise_directories() {
    let temp = tempdir().expect("temp dir");
    let product_root = temp.path().join("World of Warcraft");
    let flavor_root = product_root.join("_retail_");

    fs::create_dir_all(flavor_root.join("Interface").join("AddOns")).expect("addons dir");
    fs::create_dir_all(
        flavor_root
            .join("WTF")
            .join("Account")
            .join("ACC1")
            .join("SavedVariables"),
    )
    .expect("saved variables");
    fs::create_dir_all(
        flavor_root
            .join("WTF")
            .join("Account")
            .join("ACC1")
            .join("Illidan")
            .join("Mageone")
            .join("SavedVariables"),
    )
    .expect("valid character");
    fs::create_dir_all(flavor_root.join("WTF").join("Account").join("BROKEN")).expect("broken");
    fs::create_dir_all(
        flavor_root
            .join("WTF")
            .join("Account")
            .join("ACC1")
            .join("Illidan")
            .join("Emptychar"),
    )
    .expect("empty character placeholder");
    fs::write(
        flavor_root.join("WTF").join("Config.wtf"),
        "SET locale enUS",
    )
    .expect("config");

    let installation = inspect_installation_on_host(
        &product_root,
        Some(WowFlavor::Retail),
        HostPlatform::current(),
    )
    .expect("inspect");
    let accounts = discover_local_accounts(&installation.installation).expect("discover accounts");

    assert_eq!(accounts.len(), 1);
    assert_eq!(accounts[0].account_name, "ACC1");
    assert_eq!(accounts[0].characters.len(), 1);
    assert_eq!(accounts[0].characters[0].character, "Mageone");
}
