vk_headers = files([
  'include' / 'vulkan' / 'vulkan.h',
  'include' / 'vk_platform.h',
])

vk_sources = files([
  'src' / 'vulkan.c',
])

dl_dep = cc.find_library('dl', required: build_machine.system() != 'windows')

cflags = []
deps = [dl_dep]

if x11_enabled
  cflags += '-DVK_USE_PLATFORM_XLIB_KHR'
  deps += x11_dep
endif

if wayland_enabled
  cflags += '-DVK_USE_PLATFORM_WAYLAND_KHR'
  deps += wlclientdep
endif

libvulkan = static_library('vulkan',
  sources: [vk_sources],
  include_directories: ['include'],
  c_args: [cflags],
  dependencies: deps,
)

vulkan_dep = declare_dependency(
  include_directories: ['include'],
  link_with: [libvulkan],
)

