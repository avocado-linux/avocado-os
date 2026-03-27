project('gpio-toggle', 'c',
  version : '0.1.0',
  default_options : ['warning_level=2', 'c_std=c11'])

gpiod_dep = dependency('libgpiod', version : '>=2.1')

executable('gpio-toggle',
  'main.c',
  dependencies : [gpiod_dep],
  install : true)
