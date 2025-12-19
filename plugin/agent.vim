" Agent.nvim Plugin
" Enhanced agent interface with spec-driven development capabilities

if exists('g:loaded_agent_nvim')
  finish
endif
let g:loaded_agent_nvim = 1

" Check if Neovim version is supported
if !has('nvim-0.5.0')
  echohl ErrorMsg
  echom 'agent.nvim requires Neovim 0.5.0 or later'
  echohl None
  finish
endif

" Initialize the plugin (no default config needed - handled in Lua)
lua << EOF
require('agent').setup()
EOF

