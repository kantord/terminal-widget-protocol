-- Proof that a Neovim plugin can emit TWP APC sequences and have them
-- reach the host terminal via twp-proxy.
--
-- How to run:
--   1. From a Kitty-compatible terminal, start a wrapped shell:
--        ./twp-proxy/target/release/twp-proxy zsh
--   2. Inside that shell, launch Neovim:
--        nvim demo/twp_demo.lua
--   3. From Neovim's command line:
--        :luafile %
--
-- The trick: vim.uv.fs_write(1, ...) writes raw bytes to fd 1 (the tty),
-- bypassing Neovim's own renderer. Those bytes go up the PTY chain into
-- twp-proxy's APC filter, which renders the widget tree and emits Kitty
-- Graphics. Neovim is paused on getchar() so the widgets stay visible.
-- Press any key to let Neovim redraw.

local function emit(json)
  local apc = "\27_twp;v=1,c=20,r=4;" .. json .. "\27\\"
  vim.uv.fs_write(1, apc)
end

vim.uv.fs_write(1, "\r\n")

emit([[{"S":{"n":"flex","s":{"justify-content":"center","align-items":"center","width":"100%","height":"100%","background":"#1e293b","border-radius":24},"c":[{"n":"text","t":"Hello from Neovim!","s":{"font-size":32,"color":"#ffffff","font-weight":"bold"}}]}}]])

vim.uv.fs_write(1, "\r\n")

emit([[{"S":{"n":"flex","s":{"flex-direction":"row","justify-content":"space-around","align-items":"center","width":"100%","height":"100%","background":"#2a2d3a","border-radius":40},"c":[
  {"n":"box","s":{"width":100,"height":100,"background":"#f04646","border-radius":"50%"}},
  {"n":"box","s":{"width":100,"height":100,"background":"#fac83c","border-radius":"50%"}},
  {"n":"box","s":{"width":100,"height":100,"background":"#50dc6e","border-radius":"50%"}}
]}}]])

vim.uv.fs_write(1, "\r\n")

vim.notify(
  "TWP widgets emitted. If you can see them, the pipeline works. "
    .. "Press any key to let Neovim redraw."
)
vim.fn.getchar()
