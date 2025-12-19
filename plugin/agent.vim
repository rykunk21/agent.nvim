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

" Plugin configuration
let g:agent_nvim_config = get(g:, 'agent_nvim_config', {})

" Default configuration
let s:default_config = {
  \ 'auto_start': 0,
  \ 'keybindings': {
    \ 'open_agent': '<leader>sa',
    \ 'new_spec': '<leader>sn',
    \ 'open_spec': '<leader>so',
    \ 'close_agent': '<Esc>',
  \ },
  \ 'ui': {
    \ 'border_style': 'rounded',
    \ 'window_width_ratio': 0.8,
    \ 'window_height_ratio': 0.6,
  \ }
\ }

" Merge user config with defaults
let g:agent_nvim_config = extend(s:default_config, g:agent_nvim_config, 'force')

" Commands
command! -nargs=0 SpecAgent lua require('agent').open_agent()
command! -nargs=? SpecNew lua require('agent').new_spec(<q-args>)
command! -nargs=? SpecOpen lua require('agent').open_spec(<q-args>)
command! -nargs=0 SpecClose lua require('agent').close_agent()
command! -nargs=0 SpecStatus lua require('agent').show_status()

" Auto-commands
augroup AgentNvim
  autocmd!
  " Auto-start if configured
  if g:agent_nvim_config.auto_start
    autocmd VimEnter * lua require('agent').auto_start()
  endif
  
  " Save conversation on exit
  autocmd VimLeavePre * lua require('agent').save_state()
  
  " Handle window resize
  autocmd VimResized * lua require('agent').handle_resize()
augroup END

" Set up keybindings
function! s:setup_keybindings()
  let l:bindings = g:agent_nvim_config.keybindings
  
  if has_key(l:bindings, 'open_agent') && !empty(l:bindings.open_agent)
    execute 'nnoremap <silent> ' . l:bindings.open_agent . ' :SpecAgent<CR>'
  endif
  
  if has_key(l:bindings, 'new_spec') && !empty(l:bindings.new_spec)
    execute 'nnoremap <silent> ' . l:bindings.new_spec . ' :SpecNew<CR>'
  endif
  
  if has_key(l:bindings, 'open_spec') && !empty(l:bindings.open_spec)
    execute 'nnoremap <silent> ' . l:bindings.open_spec . ' :SpecOpen<CR>'
  endif
endfunction

call s:setup_keybindings()

