" Health check for agent.nvim plugin

function! health#agent#check() abort
  call health#report_start('agent.nvim')
  
  " Check Neovim version
  if has('nvim-0.5.0')
    call health#report_ok('Neovim version is supported')
  else
    call health#report_error('Neovim 0.5.0+ required')
  endif
  
  " Check if Rust binary exists
  if executable('nvim-spec-agent')
    call health#report_ok('Rust binary found in PATH')
  else
    " Check in plugin directory
    let l:plugin_dir = expand('<sfile>:p:h:h:h')
    let l:binary_paths = [
      \ l:plugin_dir . '/bin/nvim-spec-agent',
      \ l:plugin_dir . '/bin/nvim-spec-agent.exe',
      \ l:plugin_dir . '/target/release/nvim-spec-agent',
      \ l:plugin_dir . '/target/release/nvim-spec-agent.exe'
    \ ]
    
    let l:found = 0
    for l:path in l:binary_paths
      if executable(l:path)
        call health#report_ok('Rust binary found at: ' . l:path)
        let l:found = 1
        break
      endif
    endfor
    
    if !l:found
      call health#report_error('Rust binary not found. Run the build script or check installation.')
    endif
  endif
  
  " Check if Rust is available for building
  if executable('cargo')
    call health#report_ok('Cargo (Rust) is available for building')
  else
    call health#report_warn('Cargo not found - needed for building the plugin')
  endif
  
  " Check configuration
  if exists('g:agent_nvim_config')
    call health#report_ok('Configuration loaded')
  else
    call health#report_error('Configuration not found')
  endif
endfunction