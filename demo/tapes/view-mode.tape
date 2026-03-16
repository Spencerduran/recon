# VHS tape: recon view mode with pixel-art characters
# Run: vhs demo/view-mode.tape
# Prerequisite: ./demo/demo.sh --setup  (in another terminal)

Output demo/out/view-mode.gif
Set Shell zsh
Set FontSize 14
Set Width 1200
Set Height 700
Set Padding 10
Set Theme "Catppuccin Mocha"

# Hide the startup sequence (typing recon, table view, switching to view)
Hide
Type "recon"
Enter
Sleep 2s
Type "v"
Sleep 2s
Show

# View mode with tamagotchi characters visible from frame 1
Sleep 5s

# Let animations play
Sleep 4s

Type "q"
Sleep 500ms
