use crate::version::from_python_version;
use extism_pdk::*;
use proto_pdk::*;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;

#[host_fn]
extern "ExtismHost" {
    fn exec_command(input: Json<ExecCommandInput>) -> Json<ExecCommandOutput>;
}

static NAME: &str = "Python";

#[plugin_fn]
pub fn register_tool(Json(_): Json<ToolMetadataInput>) -> FnResult<Json<ToolMetadataOutput>> {
    Ok(Json(ToolMetadataOutput {
        name: NAME.into(),
        type_of: PluginType::Language,
        plugin_version: Some(env!("CARGO_PKG_VERSION").into()),
        ..ToolMetadataOutput::default()
    }))
}

#[derive(Deserialize)]
struct ReleaseEntry {
    download: String,
    checksum: Option<String>,
}

#[plugin_fn]
pub fn download_prebuilt(
    Json(input): Json<DownloadPrebuiltInput>,
) -> FnResult<Json<DownloadPrebuiltOutput>> {
    let env = get_proto_environment()?;

    let releases: HashMap<String, HashMap<String, ReleaseEntry>> = fetch_url_with_cache(
        "https://raw.githubusercontent.com/moonrepo/python-plugin/master/releases.json",
    )?;

    let Some(release_triples) = releases.get(&input.context.version) else {
        return err!(format!(
            "No pre-built available for version {}!",
            input.context.version
        ));
    };

    let triple = get_target_triple(&env, "Python")?;

    let Some(release) = release_triples.get(&triple) else {
        return err!(format!(
            "No pre-built available for architecture {}!",
            triple
        ));
    };

    Ok(Json(DownloadPrebuiltOutput {
        archive_prefix: Some("python".into()),
        checksum_url: release.checksum.clone(),
        download_url: release.download.clone(),
        ..DownloadPrebuiltOutput::default()
    }))
}

#[derive(Deserialize)]
struct PythonManifest {
    python_exe: String,
    python_major_minor_version: String,
}

#[plugin_fn]
pub fn locate_bins(Json(input): Json<LocateBinsInput>) -> FnResult<Json<LocateBinsOutput>> {
    let env = get_proto_environment()?;
    let mut bin_path = format_bin_name("install/bin/python3", env.os);
    let mut globals_lookup_dirs = vec!["$HOME/.local/bin".to_owned()];

    // Only available for pre-builts
    let manifest_path = input.context.tool_dir.join("PYTHON.json");

    if manifest_path.exists() {
        let manifest: PythonManifest = json::from_slice(&fs::read(manifest_path)?)?;

        bin_path = manifest.python_exe;

        if env.os == HostOS::Windows {
            let formatted_version = manifest.python_major_minor_version.replace(".", "");

            globals_lookup_dirs.push(format!(
                "$APPDATA/Roaming/Python{}/Scripts",
                formatted_version
            ));

            globals_lookup_dirs.push(format!("$APPDATA/Python{}/Scripts", formatted_version));
        }
    }

    Ok(Json(LocateBinsOutput {
        bin_path: Some(bin_path.into()),
        fallback_last_globals_dir: true,
        globals_lookup_dirs,
        ..LocateBinsOutput::default()
    }))
}

#[plugin_fn]
pub fn load_versions(Json(_): Json<LoadVersionsInput>) -> FnResult<Json<LoadVersionsOutput>> {
    let tags = load_git_tags("https://github.com/python/cpython")?;
    let tags = tags
        .into_iter()
        .filter(|t| !t.ends_with("^{}") && t != "legacy-trunk")
        .filter_map(from_python_version)
        .collect::<Vec<_>>();

    Ok(Json(LoadVersionsOutput::from(tags)?))
}

#[plugin_fn]
pub fn detect_version_files(_: ()) -> FnResult<Json<DetectVersionOutput>> {
    Ok(Json(DetectVersionOutput {
        files: vec![".python-version".into()],
    }))
}

#[plugin_fn]
pub fn create_shims(Json(_): Json<CreateShimsInput>) -> FnResult<Json<CreateShimsOutput>> {
    let mut global_shims = HashMap::new();

    global_shims.insert("pip".into(), ShimConfig::global_with_sub_command("-m pip"));

    Ok(Json(CreateShimsOutput {
        global_shims,
        ..CreateShimsOutput::default()
    }))
}

#[plugin_fn]
pub fn install_global(
    Json(input): Json<InstallGlobalInput>,
) -> FnResult<Json<InstallGlobalOutput>> {
    let result = exec_command!(inherit, "pip", ["install", "--user", &input.dependency]);

    Ok(Json(InstallGlobalOutput::from_exec_command(result)))
}

#[plugin_fn]
pub fn uninstall_global(
    Json(input): Json<UninstallGlobalInput>,
) -> FnResult<Json<UninstallGlobalOutput>> {
    let result = exec_command!(inherit, "pip", ["uninstall", "--yes", &input.dependency]);

    Ok(Json(UninstallGlobalOutput::from_exec_command(result)))
}