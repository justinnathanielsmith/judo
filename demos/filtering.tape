Output demos/filtering.gif

Set Shell "bash"
Set FontSize 18
Set Width 1200
Set Height 600
Set Padding 20

# Start Judo
Type "judo"
Enter
Sleep 2s

# Show quick filters
Type "m"
Sleep 1s
Type "t"
Sleep 1s
Type "c"
Sleep 1s

# Custom revset filter
Type "/"
Type "all()"
Enter
Sleep 2s

# Snapshot and Describe
Type "s"
Sleep 1s
Type "d"
Sleep 500ms
Type "Feat: implement recording"
Enter
Sleep 2s

# Exit
Type "q"
