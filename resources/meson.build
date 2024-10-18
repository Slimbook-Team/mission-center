pkgdatadir = join_paths(get_option('prefix'), get_option('datadir'), meson.project_name())

gnome = import('gnome')

blueprints = custom_target('blueprints',
  input: files(
    'ui/apps_page/column_header.blp',
    'ui/apps_page/list_item.blp',
    'ui/apps_page/page.blp',
    'ui/apps_page/pid_column.blp',
    'ui/apps_page/stat_column.blp',

    'ui/performance_page/cpu.blp',
    'ui/performance_page/cpu_details.blp',
    'ui/performance_page/disk.blp',
    'ui/performance_page/disk_details.blp',
    'ui/performance_page/fan.blp',
    'ui/performance_page/fan_details.blp',
    'ui/performance_page/gpu.blp',
    'ui/performance_page/gpu_details.blp',
    'ui/performance_page/memory.blp',
    'ui/performance_page/memory_details.blp',
    'ui/performance_page/memory_info_tooltip.blp',
    'ui/performance_page/network.blp',
    'ui/performance_page/network_details.blp',
    'ui/performance_page/page.blp',
    'ui/performance_page/summary_graph.blp',

    'ui/preferences/page.blp',
    'ui/preferences/window.blp',

    'ui/services_page/page.blp',
    'ui/services_page/context_menu_button.blp',
    'ui/services_page/details_dialog.blp',

    'ui/window.blp',

    'ui/gtk/help-overlay.blp',
  ),
  output: '.',
  command: [find_program('blueprint-compiler'), 'batch-compile', '@OUTPUT@', '@CURRENT_SOURCE_DIR@', '@INPUT@'],
)

gnome.compile_resources('missioncenter',
  'missioncenter.gresource.xml',
  gresource_bundle: true,
  install: true,
  install_dir: pkgdatadir,
  dependencies: blueprints,
)

