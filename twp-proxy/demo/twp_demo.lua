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
-- You should see a progress bar and a traffic light flash onto the screen.
-- Neovim is paused on getchar() so the widgets stay visible. Press any key
-- to continue — Neovim will redraw and paint over them.
--
-- The trick: vim.uv.fs_write(1, ...) writes raw bytes to fd 1 (the tty),
-- bypassing Neovim's own renderer. Those bytes go up the PTY chain into
-- twp-proxy's APC filter, which turns each `twp;` payload into a Kitty
-- Graphics transmit + Unicode placeholder block.

local function emit(payload)
  local apc = "\27_twp;" .. payload .. "\27\\"
  vim.uv.fs_write(1, apc)
end

-- Force the cursor to a fresh line so the widgets land somewhere visible.
vim.uv.fs_write(1, "\r\n")

emit("foo")
vim.uv.fs_write(1, "  ")
emit("bar")
vim.uv.fs_write(1, "\r\n")

vim.notify(
  "twp widgets emitted. If you can see them, the pipeline works. "
    .. "Press any key to let Neovim redraw."
)
vim.fn.getchar()
