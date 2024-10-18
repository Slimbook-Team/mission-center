project_name = 'gatherer'
project_path = 'src' / 'sys_info_v2' / project_name

libdrm = dependency('libdrm', required: true)
libegl = dependency('egl', required: true)
libgbm = dependency('gbm', required: true)

patch_exe = find_program('patch')

cargo_bin = find_program('cargo')
cargo_opt  = [ '--manifest-path', meson.project_source_root() / project_path / 'Cargo.toml' ]
cargo_opt += [ '--target-dir', meson.project_build_root()  / project_path / 'src' ]
cargo_env  = [ 'CARGO_HOME=' + meson.project_build_root()  / project_path/ 'cargo-home' ]

if get_option('buildtype') == 'release'
  cargo_opt += [ '--release' ]
  rust_target = 'release'
else
  rust_target = 'debug'
endif

cargo_build = custom_target(
  'cargo-build',
  build_by_default: true,
  build_always_stale: true,
  output: 'missioncenter-' + project_name,
  console: true,
  install: true,
  install_dir: get_option('bindir'),
  command: [
    'env', cargo_env,
    cargo_bin, 'build',
    cargo_opt, '&&', 'cp', project_path / 'src' / rust_target / project_name, '@OUTPUT@',
  ]
)
