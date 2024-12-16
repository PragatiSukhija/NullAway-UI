use crate::sandbox::Action::{BuildWithNullAway, Run, RunAnnotator};
use serde_derive::Deserialize;
use snafu::prelude::*;
use std::{io, os::unix::fs::PermissionsExt, path::PathBuf, string, time::Duration};
use std::path::Path;
use regex::Regex;
use tempfile::TempDir;
use tokio::{fs, process::Command, time};
use tracing::debug;
use crate::sandbox::Error::FailedToCopyFileFromContainer;

pub(crate) const DOCKER_PROCESS_TIMEOUT_SOFT: Duration = Duration::from_secs(10);
const DOCKER_PROCESS_TIMEOUT_HARD: Duration = Duration::from_secs(100);

#[derive(Debug, Deserialize)]
struct CrateInformationInner {
    name: String,
    version: String,
    id: String,
}

#[derive(Debug, Clone)]
pub struct CrateInformation {
    pub name: String,
    pub version: String,
    pub id: String,
}

impl From<CrateInformationInner> for CrateInformation {
    fn from(me: CrateInformationInner) -> Self {
        let CrateInformationInner { name, version, id } = me;
        Self { name, version, id }
    }
}

#[derive(Debug, Clone)]
pub struct Version {
    pub release: String,
    pub commit_hash: String,
    pub commit_date: String,
}

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Unable to create temporary directory: {}", source))]
    UnableToCreateTempDir { source: io::Error },
    #[snafu(display("Unable to read annotated Main.java file from the container: {}", source))]
    FailedToReadAnnotatedFile { source: io::Error },
    #[snafu(display("Unable to create output directory: {}", source))]
    UnableToCreateOutputDir { source: io::Error },
    #[snafu(display("Unable to set permissions for output directory: {}", source))]
    UnableToSetOutputPermissions { source: io::Error },
    #[snafu(display("Unable to create source file: {}", source))]
    UnableToCreateSourceFile { source: io::Error },
    #[snafu(display("Unable to set permissions for source file: {}", source))]
    UnableToSetSourcePermissions { source: io::Error },
    #[snafu(display("Failed to copy file from container: {}", source))]
    FailedToCopyFileFromContainer { source: io::Error },

    #[snafu(display("Unable to start the compiler: {}", source))]
    UnableToStartCompiler { source: io::Error },
    #[snafu(display("Unable to find the compiler ID"))]
    MissingCompilerId,
    #[snafu(display("Unable to wait for the compiler: {}", source))]
    UnableToWaitForCompiler { source: io::Error },
    #[snafu(display("Unable to get output from the compiler: {}", source))]
    UnableToGetOutputFromCompiler { source: io::Error },
    #[snafu(display("Unable to remove the compiler: {}", source))]
    UnableToRemoveCompiler { source: io::Error },
    #[snafu(display("Compiler execution took longer than {} ms", timeout.as_millis()))]
    CompilerExecutionTimedOut {
        source: tokio::time::error::Elapsed,
        timeout: Duration,
    },

    #[snafu(display("Unable to read output file: {}", source))]
    UnableToReadOutput { source: io::Error },
    #[snafu(display("Unable to read crate information: {}", source))]
    UnableToParseCrateInformation { source: ::serde_json::Error },
    #[snafu(display("Output was not valid UTF-8: {}", source))]
    OutputNotUtf8 { source: string::FromUtf8Error },
    #[snafu(display("Output was missing"))]
    OutputMissing,
    #[snafu(display("Release was missing from the version output"))]
    PackageNameMissing,
    #[snafu(display("Package name is missing or could not be parsed"))]
    VersionReleaseMissing,
    #[snafu(display("Commit hash was missing from the version output"))]
    VersionHashMissing,
    #[snafu(display("Commit date was missing from the version output"))]
    VersionDateMissing,
}

pub type Result<T, E = Error> = ::std::result::Result<T, E>;

fn vec_to_str(v: Vec<u8>) -> Result<String> {
    String::from_utf8(v).context(OutputNotUtf8Snafu)
}

// We must create a world-writable files (rustfmt) and directories
// (LLVM IR) so that the process inside the Docker container can write
// into it.
//
// This problem does *not* occur when using the indirection of
// docker-machine.
fn wide_open_permissions() -> std::fs::Permissions {
    PermissionsExt::from_mode(0o777)
}

macro_rules! docker_command {
    ($($arg:expr),* $(,)?) => ({
        let mut cmd = Command::new("docker");
        $( cmd.arg($arg); )*
        cmd
    });
}

fn basic_secure_docker_command() -> Command {
    let mut cmd = docker_command!(
        "run",
        "--platform",
        "linux/amd64",
        "--detach",
        "--cap-drop=ALL",
        // Needed to allow overwriting the file
        "--cap-add=DAC_OVERRIDE",
        "--security-opt=no-new-privileges",
        "--workdir",
        "/playground",
        "--net",
        "none",
        "--memory",
        "512m",
        "--memory-swap",
        "640m",
        "--env",
        format!(
            "PLAYGROUND_TIMEOUT={}",
            DOCKER_PROCESS_TIMEOUT_SOFT.as_secs()
        ),
    );

    cmd.args(&["--pids-limit", "512"]);

    cmd.kill_on_drop(true);

    cmd
}

fn build_execution_command(
    req: &(impl ActionRequest + PreviewRequest + ReleaseRequest + RuntimeRequest + NullAwayConfigDataRequest + AnnotatorConfigRequest),
    package_name: &str,
) -> Vec<String> {
    use self::Action::*;

    let mut cmd: Vec<String> = vec![];

    let release = req
        .release()
        .unwrap_or(req.runtime().default_release())
        .java_release();

    let action = req.action();


    if action == Run {
        cmd.push("java".to_string());
        cmd.extend(["--module-path".to_string(), "dependencies".to_string()]);
        cmd.extend(["--add-modules".to_string(), "ALL-MODULE-PATH".to_string()]);
        cmd.extend(["--source".to_string(), release.to_string()]);

        // Enable using java.lang.foreign w/o warnings
        // cmd.push("--enable-native-access=ALL-UNNAMED".to_string());

        if req.preview() {
            cmd.push("--enable-preview".to_string());
        }

        cmd.push("Main.java".to_string());
    } else if action == BuildWithNullAway{

        cmd.push("javac".to_string());
        cmd.extend(["--module-path".to_string(), "dependencies".to_string()]);
        cmd.extend(["--add-modules".to_string(), "ALL-MODULE-PATH".to_string()]);
        cmd.extend(["--release".to_string(), release.to_string()]);
        cmd.extend(["-d".to_string(), "out".to_string()]);

        if req.preview() {
            cmd.push("--enable-preview".to_string());
        }

        //Added for Error-prone integration
        cmd.extend([
            "-J--add-exports=jdk.compiler/com.sun.tools.javac.api=ALL-UNNAMED".to_string(),
            "-J--add-exports=jdk.compiler/com.sun.tools.javac.file=ALL-UNNAMED".to_string(),
            "-J--add-exports=jdk.compiler/com.sun.tools.javac.main=ALL-UNNAMED".to_string(),
            "-J--add-exports=jdk.compiler/com.sun.tools.javac.model=ALL-UNNAMED".to_string(),
            "-J--add-exports=jdk.compiler/com.sun.tools.javac.parser=ALL-UNNAMED".to_string(),
            "-J--add-exports=jdk.compiler/com.sun.tools.javac.processing=ALL-UNNAMED".to_string(),
            "-J--add-exports=jdk.compiler/com.sun.tools.javac.tree=ALL-UNNAMED".to_string(),
            "-J--add-exports=jdk.compiler/com.sun.tools.javac.util=ALL-UNNAMED".to_string(),
            "-J--add-opens=jdk.compiler/com.sun.tools.javac.code=ALL-UNNAMED".to_string(),
            "-J--add-opens=jdk.compiler/com.sun.tools.javac.comp=ALL-UNNAMED".to_string(),
            "-XDcompilePolicy=simple".to_string(),
            "-processorpath".to_string(),
            "plugins/error_prone_core-2.32.0-with-dependencies.jar:plugins/dataflow-errorprone-3.42.0-eisop4.jar:plugins/nullaway-0.12.2.jar:plugins/jspecify-1.0.0.jar:plugins/dataflow-nullaway-3.47.0.jar:plugins/checker-qual-3.9.1.jar:plugins/jsr305-3.0.2.jar".to_string()
        ]);

        let mut nullaway_options = String::from("-Xplugin:ErrorProne -XepDisableAllChecks -Xep:NullAway:ERROR");

        if !package_name.is_empty() {
            nullaway_options.push_str(&format!(" -XepOpt:NullAway:AnnotatedPackages={}", package_name));
        }

        if let Some(nullaway_config_data) = req.nullaway_config_data() {

            if let Some(cast_method) = &nullaway_config_data.cast_to_non_null_method {
                if !cast_method.is_empty() {
                    nullaway_options.push_str(&format!(" -XepOpt:NullAway:CastToNonNullMethod={}", cast_method));
                }
            }

            if nullaway_config_data.check_optional_emptiness {
                nullaway_options.push_str(" -XepOpt:NullAway:CheckOptionalEmptiness=true");
            }

            if nullaway_config_data.check_contracts {
                nullaway_options.push_str(" -XepOpt:NullAway:CheckContracts=true");
            }

            if nullaway_config_data.j_specify_mode {
                nullaway_options.push_str(" -XepOpt:NullAway:JSpecifyMode=true");
            }
        }

        cmd.extend([nullaway_options]);

        cmd.push("Main.java".to_string());

        //println!("{:?}", cmd);

    }else if action == RunAnnotator {
        cmd.push("sh".to_string());
        cmd.push("-c".to_string());

        // Base Java command
        let mut java_command = format!(
            "java -jar plugins/annotator-core-1.3.15.jar \
        -d playground-result/ \
        -cp config/paths.tsv \
        -i com.example.Initializer \
        -cn NULLAWAY \
        -bc 'sh -c \"javac \
            --module-path dependencies \
            --add-modules ALL-MODULE-PATH \
            -d output/ \
            -J--add-exports=jdk.compiler/com.sun.tools.javac.api=ALL-UNNAMED \
            -J--add-exports=jdk.compiler/com.sun.tools.javac.file=ALL-UNNAMED \
            -J--add-exports=jdk.compiler/com.sun.tools.javac.main=ALL-UNNAMED \
            -J--add-exports=jdk.compiler/com.sun.tools.javac.model=ALL-UNNAMED \
            -J--add-exports=jdk.compiler/com.sun.tools.javac.parser=ALL-UNNAMED \
            -J--add-exports=jdk.compiler/com.sun.tools.javac.processing=ALL-UNNAMED \
            -J--add-exports=jdk.compiler/com.sun.tools.javac.tree=ALL-UNNAMED \
            -J--add-exports=jdk.compiler/com.sun.tools.javac.util=ALL-UNNAMED \
            -J--add-opens=jdk.compiler/com.sun.tools.javac.code=ALL-UNNAMED \
            -J--add-opens=jdk.compiler/com.sun.tools.javac.comp=ALL-UNNAMED \
            -XDcompilePolicy=simple \
            -processorpath {processor_path} \
            -Xplugin:\\\"ErrorProne \
            -Xep:NullAway:ERROR \
            -Xep:AnnotatorScanner:ERROR \
            -XepOpt:NullAway:AnnotatedPackages={package} \
            -XepOpt:NullAway:SerializeFixMetadata=true \
            -XepOpt:NullAway:FixSerializationConfigPath=config/nullaway.xml \
            -XepOpt:AnnotatorScanner:ConfigPath=config/scanner.xml\\\" \
            Main.java\"' --nullable org.jspecify.annotations.Nullable",
            processor_path = "plugins/error_prone_core-2.32.0-with-dependencies.jar:\
            plugins/dataflow-errorprone-3.42.0-eisop4.jar:\
            plugins/nullaway-0.12.2.jar:\
            plugins/jspecify-1.0.0.jar:\
            plugins/dataflow-nullaway-3.47.0.jar:\
            plugins/checker-qual-3.9.1.jar:\
            plugins/jsr305-3.0.2.jar:\
            plugins/annotator-scanner-1.3.15.jar",
            package = package_name
        );


        if let Some(annotator_config) = req.annotator_config() {
            if annotator_config.nullUnmarked {
                java_command.push_str(" -sre org.jspecify.annotations.NullUnmarked");
            }
        }

        java_command.push_str(" > /dev/null 2>&1 && cat Main.java");
        cmd.push(java_command);

    }else if action==Build{
        cmd.push("javac".to_string());
        cmd.extend(["--module-path".to_string(), "dependencies".to_string()]);
        cmd.extend(["--add-modules".to_string(), "ALL-MODULE-PATH".to_string()]);
        cmd.extend(["--release".to_string(), release.to_string()]);
        cmd.extend(["-d".to_string(), "out".to_string()]);

        if req.preview() {
            cmd.push("--enable-preview".to_string());
        }

        cmd.push("Main.java".to_string());
    }

    cmd
}

pub struct Sandbox {
    #[allow(dead_code)]
    scratch: TempDir,
    input_file: PathBuf,
    output_dir: PathBuf,
}

impl Sandbox {
    pub async fn new() -> Result<Self> {
        // `TempDir` performs *synchronous* filesystem operations
        // now and when it's dropped. We accept that under the
        // assumption that the specific operations will be quick
        // enough.
        let scratch = tempfile::Builder::new()
            .prefix("playground")
            .tempdir()
            .context(UnableToCreateTempDirSnafu)?;
        let input_file = scratch.path().join("input.rs");
        let output_dir = scratch.path().join("output");

        fs::create_dir(&output_dir)
            .await
            .context(UnableToCreateOutputDirSnafu)?;
        fs::set_permissions(&output_dir, wide_open_permissions())
            .await
            .context(UnableToSetOutputPermissionsSnafu)?;

        Ok(Sandbox {
            scratch,
            input_file,
            output_dir,
        })
    }

    pub async fn execute(&self, req: &ExecuteRequest) -> Result<ExecuteResponse> {
        self.write_source_code(&req.code).await?;

        let package_name = self.extract_package_name(&req.code);
        let action = req.action();

        if (action == BuildWithNullAway || action == RunAnnotator) && package_name.is_empty() {
            return Ok(ExecuteResponse {
                success: false,
                stdout: String::new(),
                stderr: "Error: Package name is either missing or could not be parsed. Please specify a valid package name.\n\
                NullAway and NullAway Annotator require valid package declarations.".to_string(),
            });
        }

        let command = self.execute_command(req, &package_name);
        //println!("Running command: {:?}", command);

        let output = run_command_with_timeout(command).await?;

        Ok(ExecuteResponse {
            success: output.status.success(),
            stdout: vec_to_str(output.stdout)?,
            stderr: vec_to_str(output.stderr)?,
        })
    }

    fn extract_package_name(&self, code: &str) -> String {

        let package_regex = Regex::new(r"(?m)^\s*package\s+([\w\.]+);");

        if let Ok(regex) = package_regex {

            if let Some(captures) = regex.captures(code) {
                if let Some(package_name) = captures.get(1) {
                    return package_name.as_str().to_string();
                }
            }
        }

        String::new()
    }

    pub async fn crates(&self) -> Result<Vec<CrateInformation>> {
        /* let mut command = basic_secure_docker_command();
        command.args(&[Runtime::Stable.container_name()]);
        command.args(&["cat", "crate-information.json"]);

        let output = run_command_with_timeout(command).await?;

        let crate_info: Vec<CrateInformationInner> =
            ::serde_json::from_slice(&output.stdout).context(UnableToParseCrateInformationSnafu)?;

        let crates = crate_info.into_iter().map(Into::into).collect();
        Ok(crates)
        */

        Ok(vec![])
    }

    pub async fn version(&self, runtime: Runtime) -> Result<Version> {
        let mut command = basic_secure_docker_command();
        command.args(&[runtime.container_name()]);
        command.args(&["java", "--version"]);

        let output = run_command_with_timeout(command).await?;

        let version_output = vec_to_str(output.stdout)?;

        let version = version_output
            .lines()
            .take(1)
            .fold(String::new(), |a, b| a + " " + b);

        Ok(Version {
            release: version.trim().to_string(),
            commit_hash: version.trim().to_string(),
            commit_date: version.trim().to_string(),
        })
    }

    async fn write_source_code(&self, code: &str) -> Result<()> {
        fs::write(&self.input_file, code)
            .await
            .context(UnableToCreateSourceFileSnafu)?;
        fs::set_permissions(&self.input_file, wide_open_permissions())
            .await
            .context(UnableToSetSourcePermissionsSnafu)?;

        debug!(
            "Wrote {} bytes of source to {}",
            code.len(),
            self.input_file.display()
        );
        Ok(())
    }

    fn execute_command(
        &self,
        req: impl ActionRequest + ReleaseRequest + PreviewRequest + RuntimeRequest + NullAwayConfigDataRequest + AnnotatorConfigRequest,
        package_name: &str,
    ) -> Command {
        let mut cmd = self.docker_command(Some(req.action()));

        let execution_cmd = build_execution_command(&req,package_name);

        cmd.arg(&req.runtime().container_name())
            .args(&execution_cmd);

        debug!("Execution command is {:?}", cmd);

        cmd
    }

    fn docker_command(&self, action: Option<Action>) -> Command {
        let action = action.unwrap_or(Run);

        let mut mount_input_file = self.input_file.as_os_str().to_os_string();
        mount_input_file.push(":");
        mount_input_file.push("/playground/");
        mount_input_file.push(action.file_name());

        let mut mount_output_dir = self.output_dir.as_os_str().to_os_string();
        mount_output_dir.push(":");
        mount_output_dir.push("/playground-result");

        let mut cmd = basic_secure_docker_command();

        cmd.arg("--volume")
            .arg(&mount_input_file)
            .arg("--volume")
            .arg(&mount_output_dir);

        cmd
    }
}

async fn run_command_with_timeout(mut command: Command) -> Result<std::process::Output> {

    //println!("Running command: {:?}", command);

    use std::os::unix::process::ExitStatusExt;

    let timeout = DOCKER_PROCESS_TIMEOUT_HARD;

    let output = command.output().await.context(UnableToStartCompilerSnafu)?;


    // Exit early, in case we don't have the container
    if !output.status.success() {
        return Ok(output);
    }

    let output = String::from_utf8_lossy(&output.stdout);
    let id = output
        .lines()
        .next()
        .context(MissingCompilerIdSnafu)?
        .trim();

    // ----------

    let mut command = docker_command!("wait", id);

    let timed_out = match time::timeout(timeout, command.output()).await {
        Ok(Ok(o)) => {
            // Didn't time out, didn't fail to run
            let o = String::from_utf8_lossy(&o.stdout);
            let code = o
                .lines()
                .next()
                .unwrap_or("")
                .trim()
                .parse()
                .unwrap_or(i32::MAX);
            Ok(ExitStatusExt::from_raw(code))
        }
        Ok(e) => return e.context(UnableToWaitForCompilerSnafu), // Failed to run
        Err(e) => Err(e),                                        // Timed out
    };

    // ----------

    let mut command = docker_command!("logs", id);
    let mut output = command
        .output()
        .await
        .context(UnableToGetOutputFromCompilerSnafu)?;

    // ----------

    //Adding this to copy annotated Main.java file to working directory.
    /*
    let local_path = "frontend/Main.java";
    let container_path = format!("{}:playground/Main.java", id);
    let mut copy_command = Command::new("docker");
    copy_command.args(["cp", &container_path, local_path]);
    let _ = copy_command
        .output()
        .await
        .context(FailedToCopyFileFromContainerSnafu)?;

    */



    let mut command = docker_command!(
        "rm", // Kills container if still running
        "--force", id
    );
    command.stdout(std::process::Stdio::null());
    command
        .status()
        .await
        .context(UnableToRemoveCompilerSnafu)?;

    let code = timed_out.context(CompilerExecutionTimedOutSnafu { timeout })?;

    output.status = code;

    Ok(output)
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, strum::IntoStaticStr)]
pub enum Runtime {
    Latest,
    Valhalla,
    EarlyAccess,
}

impl Runtime {
    fn default_release(&self) -> Release {
        match *self {
            Runtime::Latest => Release::_22,
            Runtime::Valhalla => Release::_20,
            Runtime::EarlyAccess => Release::_23,
        }
    }

    fn container_name(&self) -> &'static str {
        use self::Runtime::*;

        match *self {
            Latest => "javaplayground/latest",
            Valhalla => "javaplayground/valhalla",
            EarlyAccess => "javaplayground/early_access",
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, strum::IntoStaticStr)]
pub enum Release {
    _8,
    _9,
    _10,
    _11,
    _12,
    _13,
    _14,
    _15,
    _16,
    _17,
    _18,
    _19,
    _20,
    _21,
    _22,
    _23,
}

impl Release {
    fn java_release(&self) -> &'static str {
        use self::Release::*;

        match *self {
            _8 => "8",
            _9 => "9",
            _10 => "10",
            _11 => "11",
            _12 => "12",
            _13 => "13",
            _14 => "14",
            _15 => "15",
            _16 => "16",
            _17 => "17",
            _18 => "18",
            _19 => "19",
            _20 => "20",
            _21 => "21",
            _22 => "22",
            _23 => "23",
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, strum::IntoStaticStr)]
pub enum Action {
    Run,
    Build,
    BuildWithNullAway,
    RunAnnotator
}

impl Action {
    fn file_name(&self) -> &'static str {
        "Main.java"
    }
}

trait DockerCommandExt {
    fn apply_release(&mut self, req: impl ReleaseRequest);
}

impl DockerCommandExt for Command {
    fn apply_release(&mut self, req: impl ReleaseRequest) {
        if let Some(release) = req.release() {
            self.args(&["--release", release.java_release()]);
        }
    }
}

trait ActionRequest {
    fn action(&self) -> Action;
}

impl<R: ActionRequest> ActionRequest for &'_ R {
    fn action(&self) -> Action {
        (*self).action()
    }
}



trait ReleaseRequest {
    fn release(&self) -> Option<Release>;
}

impl<R: ReleaseRequest> ReleaseRequest for &'_ R {
    fn release(&self) -> Option<Release> {
        (*self).release()
    }
}

trait PreviewRequest {
    fn preview(&self) -> bool;
}

impl<R: PreviewRequest> PreviewRequest for &'_ R {
    fn preview(&self) -> bool {
        (*self).preview()
    }
}

trait RuntimeRequest {
    fn runtime(&self) -> Runtime;
}

pub trait NullAwayConfigDataRequest {
    fn nullaway_config_data(&self) -> Option<&NullAwayConfigData>;
}

pub trait AnnotatorConfigRequest {
    fn annotator_config(&self) -> Option<&AnnotatorConfig>;
}


impl<R: RuntimeRequest> RuntimeRequest for &'_ R {
    fn runtime(&self) -> Runtime {
        (*self).runtime()
    }
}

#[derive(Debug, Clone)]
pub struct CompileRequest {
    pub runtime: Runtime,
    pub action: Action,
    pub release: Option<Release>,
    pub preview: bool,
    pub code: String,
    pub nullaway_config_data: Option<NullAwayConfigData>,
    pub annotator_config: Option<AnnotatorConfig>,

}

impl ActionRequest for CompileRequest {
    fn action(&self) -> Action {
        self.action
    }
}
impl NullAwayConfigDataRequest for CompileRequest {
    fn nullaway_config_data(&self) -> Option<&NullAwayConfigData> {
        self.nullaway_config_data.as_ref()
    }
}

impl AnnotatorConfigRequest for CompileRequest {
    fn annotator_config(&self) -> Option<&AnnotatorConfig> {
        self.annotator_config.as_ref()
    }
}

impl ReleaseRequest for CompileRequest {
    fn release(&self) -> Option<Release> {
        self.release
    }
}

impl PreviewRequest for CompileRequest {
    fn preview(&self) -> bool {
        self.preview
    }
}

impl RuntimeRequest for CompileRequest {
    fn runtime(&self) -> Runtime {
        self.runtime
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct NullAwayConfigData {
    #[serde(rename = "castToNonNullMethod")]
    pub cast_to_non_null_method: Option<String>,

    #[serde(rename = "checkOptionalEmptiness")]
    pub check_optional_emptiness: bool,

    #[serde(rename = "checkContracts")]
    pub check_contracts: bool,

    #[serde(rename = "jSpecifyMode")]
    pub j_specify_mode: bool,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AnnotatorConfig {
    #[serde(rename = "nullUnmarked")]
    pub nullUnmarked: bool,
}

#[derive(Debug, Clone)]
pub struct CompileResponse {
    pub success: bool,
    pub code: String,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, Clone)]
pub struct ExecuteRequest {
    pub runtime: Runtime,
    pub release: Option<Release>,
    pub action: Action,
    pub preview: bool,
    pub code: String,
    pub nullaway_config_data: Option<NullAwayConfigData>,
    pub annotator_config: Option<AnnotatorConfig>,
}

impl ActionRequest for ExecuteRequest {
    fn action(&self) -> Action {
        self.action
    }
}

impl NullAwayConfigDataRequest for &ExecuteRequest  {
    fn nullaway_config_data(&self) -> Option<&NullAwayConfigData> {
        self.nullaway_config_data.as_ref()
    }
}

impl AnnotatorConfigRequest for &ExecuteRequest  {
    fn annotator_config(&self) -> Option<&AnnotatorConfig> {
        self.annotator_config.as_ref()
    }
}


impl ReleaseRequest for ExecuteRequest {
    fn release(&self) -> Option<Release> {
        self.release
    }
}

impl PreviewRequest for ExecuteRequest {
    fn preview(&self) -> bool {
        self.preview
    }
}

impl RuntimeRequest for ExecuteRequest {
    fn runtime(&self) -> Runtime {
        self.runtime
    }
}


#[derive(Debug, Clone)]
pub struct ExecuteResponse {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::sandbox::Error::CompilerExecutionTimedOut;

    // Running the tests completely in parallel causes spurious
    // failures due to my resource-limited Docker
    // environment. Additionally, we have some tests that *require*
    // that no other Docker processes are running.
    fn one_test_at_a_time() -> impl Drop {
        use lazy_static::lazy_static;
        use std::sync::Mutex;

        lazy_static! {
            static ref DOCKER_SINGLETON: Mutex<()> = Default::default();
        }

        // We can't poison the empty tuple
        DOCKER_SINGLETON.lock().unwrap_or_else(|e| e.into_inner())
    }

    const HELLO_WORLD_CODE: &'static str = r#"
    public class Main {
      public static void main(String[] args) {
        System.out.println("Hello, world!");
      }
    }
}
    "#;

    impl Default for ExecuteRequest {
        fn default() -> Self {
            ExecuteRequest {
                runtime: Runtime::Latest,
                action: Action::Run,
                code: HELLO_WORLD_CODE.to_string(),
                release: None,
                preview: false,
                nullaway_config_data: None,
                annotator_config: None,
            }
        }
    }

    impl Default for CompileRequest {
        fn default() -> Self {
            CompileRequest {
                runtime: Runtime::Latest,
                action: Action::Run,
                code: HELLO_WORLD_CODE.to_string(),
                release: None,
                preview: false,
                nullaway_config_data: None,
                annotator_config: None,
            }
        }
    }

    #[tokio::test]
    async fn basic_functionality() {
        let _singleton = one_test_at_a_time();
        let req = ExecuteRequest::default();

        let sb = Sandbox::new().await.expect("Unable to create sandbox");
        let resp = sb.execute(&req).await.expect("Unable to execute code");

        assert!(resp.stdout.contains("Hello, world!"));
    }

    #[tokio::test]
    async fn network_connections_are_disabled() {
        let _singleton = one_test_at_a_time();
        let code = r#"
            import java.net.URL;

            public class Main {
                public static void main(String[] args) {
                   try {
                       new URL("https://google.com:443").openStream().readAllBytes();
                       System.out.println("Able to connect to the outside world");
                   } catch (Exception e) {
                      System.out.println("Failed to connect " + e);
                   }
                }
            }
        "#;

        let req = ExecuteRequest {
            code: code.to_string(),
            ..ExecuteRequest::default()
        };

        let sb = Sandbox::new().await.expect("Unable to create sandbox");
        let resp = sb.execute(&req).await.expect("Unable to execute code");
        assert!(resp.stdout.contains("Failed to connect"));
    }

    #[tokio::test]
    async fn memory_usage_is_limited() {
        let _singleton = one_test_at_a_time();
        let code = r#"
            public class Main {
                public static void main(String[] args) {
                   int gigabyte = 1024 * 1024 * 1024;
                   var big = new int[gigabyte];
                   for (int i = 0; i < big.length; i++) { big[i] = big[i] + 1; }
                }
            }
        "#;

        let req = ExecuteRequest {
            code: code.to_string(),
            ..ExecuteRequest::default()
        };

        let sb = Sandbox::new().await.expect("Unable to create sandbox");
        let resp = sb.execute(&req).await.expect("Unable to execute code");

        assert!(
            resp.stderr.contains("java.lang.OutOfMemoryError"),
            "was: {}",
            resp.stderr
        );
    }

    #[tokio::test]
    async fn memory_usage_is_limited_even_with_bytebuffer() {
        let _singleton = one_test_at_a_time();
        let code = r#"
            import java.nio.ByteBuffer;
            public class Main {
                public static void main(String[] args) {
                   int gigabyte = 1024 * 1024 * 1024;
                   var byteBuffer = ByteBuffer.allocate(gigabyte);
                }
            }
        "#;

        let req = ExecuteRequest {
            code: code.to_string(),
            ..ExecuteRequest::default()
        };

        let sb = Sandbox::new().await.expect("Unable to create sandbox");
        let resp = sb.execute(&req).await.expect("Unable to execute code");

        assert!(
            resp.stderr.contains("java.lang.OutOfMemoryError"),
            "was: {}",
            resp.stderr
        );
    }

    #[tokio::test]
    async fn wallclock_time_is_limited() {
        let _singleton = one_test_at_a_time();
        let code = r#"
            public class Main {
                public static void main(String[] args) throws Exception {
                    Thread.sleep(20000000);
                }
            }
        "#;

        let req = ExecuteRequest {
            code: code.to_string(),
            ..ExecuteRequest::default()
        };

        let sb = Sandbox::new().await.expect("Unable to create sandbox");
        let resp = sb.execute(&req).await;

        assert!(match resp {
            Err(CompilerExecutionTimedOut {
                timeout: DOCKER_PROCESS_TIMEOUT_HARD,
                ..
            }) => {
                true
            }
            Ok(_) | Err(_) => {
                false
            }
        });
    }

    #[tokio::test]
    async fn number_of_pids_is_limited() {
        let _singleton = one_test_at_a_time();
        let forkbomb = r##"
import java.util.List;
public class Main {
  public static void main(String[] args) throws Exception {
    new ProcessBuilder(List.of(
		"sh",
        "-c",
        "z() {\n" +
                   "     z&\n" +
                   "     z\n" +
                   " }\n" +
                   " z"
    )).start().waitFor();
  }
}
        "##;

        let req = ExecuteRequest {
            code: forkbomb.to_string(),
            ..ExecuteRequest::default()
        };

        let sb = Sandbox::new().await.expect("Unable to create sandbox");
        let resp = sb.execute(&req).await.expect("Unable to execute code");

        assert!(resp.stderr.contains("unable to create native thread: possibly out of memory or process/resource limits reached"));
    }
}
