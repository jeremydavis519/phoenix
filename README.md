# phoenix

Details coming sometime. Probably not soon.

## Git hooks

Some Git hooks are stored in this repository to ensure that new commits don't obviously break
anything. But since Git doesn't install them automatically and we currently don't have a single
makefile for the entire project, you will have to install the hooks yourself.

If you're using Git 2.9 or later, this command (run from the repository's root directory) will do
the trick:
```
git config core.hooksPath .githooks
```

Earlier versions of Git require a more complicated command (also from the repository's root
directory). Make sure any custom hooks you've set up are moved, renamed, or deleted, then run this:
```
find .githooks -type f -exec ln -sf ../../{} .git/hooks/ \;
```
