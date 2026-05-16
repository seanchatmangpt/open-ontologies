//! Erlang/OTP target generator — supervision tree.
//!
//! Emits `erlang/src/<name>_app.erl`, `erlang/src/<name>_sup.erl`,
//! `erlang/src/<name>_worker.erl`, `erlang/rebar.config`. The
//! supervision tree carries `supervisor_children` workers under a
//! `one_for_one` strategy.

use super::{with_header, ManufacturedFile, SolutionSpec};

/// Generate Erlang/OTP supervision-tree files for the given spec.
///
/// Always returns exactly four files: `_app.erl`, `_sup.erl`, `_worker.erl`,
/// and `rebar.config`. The supervisor initialises `supervisor_children`
/// child specs under a `one_for_one` strategy.
///
/// # Examples
///
/// ```
/// use open_ontologies::manufacturing::{erlang, SolutionSpec};
///
/// let spec = SolutionSpec {
///     name: "revops".into(),
///     description: "RevOps pipeline".into(),
///     iac_target: "aws".into(),
///     region: "eu-west-1".into(),
///     supervisor_children: 3,
///     mcu_target: "esp32".into(),
///     work_order_receipt_hash: "c".repeat(64),
/// };
/// let files = erlang::generate(&spec);
/// assert_eq!(files.len(), 4);
///
/// // All four expected file suffixes are present.
/// let paths: Vec<&str> = files.iter().map(|f| f.path.as_str()).collect();
/// assert!(paths.iter().any(|p| p.ends_with("_app.erl")));
/// assert!(paths.iter().any(|p| p.ends_with("_sup.erl")));
/// assert!(paths.iter().any(|p| p.ends_with("_worker.erl")));
/// assert!(paths.iter().any(|p| p.ends_with("rebar.config")));
///
/// // Supervisor file contains correct child count.
/// let sup = files.iter().find(|f| f.path.ends_with("_sup.erl")).unwrap();
/// assert!(sup.contents.contains("worker_2"));
///
/// // All files are tagged for the "erlang" target.
/// assert!(files.iter().all(|f| f.target == "erlang"));
/// ```
pub fn generate(spec: &SolutionSpec) -> Vec<ManufacturedFile> {
    vec![
        file(
            &format!("erlang/src/{}_app.erl", spec.name),
            &generate_app_erl(spec),
            spec,
        ),
        file(
            &format!("erlang/src/{}_sup.erl", spec.name),
            &generate_sup_erl(spec),
            spec,
        ),
        file(
            &format!("erlang/src/{}_worker.erl", spec.name),
            &generate_worker_erl(spec),
            spec,
        ),
        file("erlang/rebar.config", &generate_rebar_config(spec), spec),
    ]
}

fn file(path: &str, body: &str, spec: &SolutionSpec) -> ManufacturedFile {
    ManufacturedFile {
        path: path.to_string(),
        contents: with_header(spec, path, body),
        target: "erlang".to_string(),
    }
}

fn generate_app_erl(spec: &SolutionSpec) -> String {
    format!(
        "-module({name}_app).\n\
         -behaviour(application).\n\
         \n\
         -export([start/2, stop/1]).\n\
         \n\
         start(_StartType, _StartArgs) ->\n\
         \x20\x20\x20\x20{name}_sup:start_link().\n\
         \n\
         stop(_State) ->\n\
         \x20\x20\x20\x20ok.\n",
        name = spec.name,
    )
}

fn generate_sup_erl(spec: &SolutionSpec) -> String {
    let children = (0..spec.supervisor_children)
        .map(|i| {
            format!(
                "        #{{id => worker_{i}, start => {{{name}_worker, start_link, [{i}]}}, restart => permanent, shutdown => 5000, type => worker, modules => [{name}_worker]}}",
                i = i,
                name = spec.name
            )
        })
        .collect::<Vec<_>>()
        .join(",\n");
    format!(
        "-module({name}_sup).\n\
         -behaviour(supervisor).\n\
         \n\
         -export([start_link/0, init/1]).\n\
         \n\
         -define(WORK_ORDER_RECEIPT, \"{wor}\").\n\
         \n\
         start_link() ->\n\
         \x20\x20\x20\x20supervisor:start_link({{local, ?MODULE}}, ?MODULE, []).\n\
         \n\
         init([]) ->\n\
         \x20\x20\x20\x20SupFlags = #{{strategy => one_for_one, intensity => 5, period => 10}},\n\
         \x20\x20\x20\x20Children = [\n\
{children}\n\
         \x20\x20\x20\x20],\n\
         \x20\x20\x20\x20{{ok, {{SupFlags, Children}}}}.\n",
        name = spec.name,
        wor = spec.work_order_receipt_hash,
        children = children,
    )
}

fn generate_worker_erl(spec: &SolutionSpec) -> String {
    format!(
        "-module({name}_worker).\n\
         -behaviour(gen_server).\n\
         \n\
         -export([start_link/1]).\n\
         -export([init/1, handle_call/3, handle_cast/2, handle_info/2, terminate/2, code_change/3]).\n\
         \n\
         start_link(Id) ->\n\
         \x20\x20\x20\x20gen_server:start_link(?MODULE, [Id], []).\n\
         \n\
         init([Id]) ->\n\
         \x20\x20\x20\x20{{ok, #{{id => Id}}}}.\n\
         \n\
         handle_call(_Request, _From, State) ->\n\
         \x20\x20\x20\x20{{reply, ok, State}}.\n\
         \n\
         handle_cast(_Msg, State) ->\n\
         \x20\x20\x20\x20{{noreply, State}}.\n\
         \n\
         handle_info(_Info, State) ->\n\
         \x20\x20\x20\x20{{noreply, State}}.\n\
         \n\
         terminate(_Reason, _State) ->\n\
         \x20\x20\x20\x20ok.\n\
         \n\
         code_change(_OldVsn, State, _Extra) ->\n\
         \x20\x20\x20\x20{{ok, State}}.\n",
        name = spec.name,
    )
}

fn generate_rebar_config(spec: &SolutionSpec) -> String {
    format!(
        "{{erl_opts, [debug_info]}}.\n\
         {{deps, []}}.\n\
         {{relx, [\n\
         \x20\x20\x20\x20{{release, {{{name}, \"0.1.0\"}}, [{name}]}},\n\
         \x20\x20\x20\x20{{dev_mode, true}},\n\
         \x20\x20\x20\x20{{include_erts, false}}\n\
         ]}}.\n",
        name = spec.name,
    )
}
