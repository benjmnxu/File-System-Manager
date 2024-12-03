The implemeneted project is file management tool for MacOS (only MacOS). The product primarily seeks to de-abstract the APFS (Apple File System) in order for users to directly locate and manage files. Cleanups can either be manually or AI-powered.

Once cloned, there are three CLI flags to consider: `--gui-mode`, `--action-file`, and `--dry`.

`--gui-mode`: Including this flag launches the application with a gui. Excluding it launches the application with only the terminal interface. AI features are only available when using the gui interface.

`--action-file`: Including this flag creates a action file where all committed actions are stored.

`--dry`: Including this flag launches the application in dry mode–committed actions are not actually passed onto the computer's file system. All changes are only virtual.

It may also be importand to consider whether or not to run this application with `sudo`. This is highly dependent on your own filesystem and permissions setup.

The first step should always be specifying the root on which the application should run (to scope the entire filesystem, input '/'). On the gui, do this by navigating to the Load Page and clicking 'load'. Using the cli, simply enter the desired location. 

On gui:

![performance image](media/gui-load.png "Optional title")

On cli:

![performance image](media/cli-load.png "Optional title")

This file load should be relatively quick depending on the size of your specified location and computer. On my hardware, I get the following results:

![performance image](media/performance.png "Optional title")

As a sidenote, this is achieved using rayon and C-bindings. Using parallelized Rust-native WalkDir takes significantly longer (around 10 minutes).


After loading the filesystem, you will have access to the following commands:
```
1. `..` - Moves up one level.
2. `<index>` - Moves down to the child at the specified index.
3. `go to <path>` - Navigates to the specified path.
4. `commit` - Commits the current state.
5. `undo <index>` - Reverts to a specific commit index.
6. `status` - Displays the current status.
7. `display` - Displays content or structure at the current level.
8. `create file <name>` - Creates a file with the specified name.
9. `create folder <name>` - Creates a folder with the specified name.
10. `del <index>` - Deletes the item at the specified index.
11. `open <index>` - Opens the item at the specified index.
12. `move <source> > <destination>` - Moves an item from source into destination directory.
13. `help` - Displays this help message.
```

`<index>` refers to an integer while all others should be Strings.

These commands allow you to freely crawl through and manipulate your local file system. Every change you command will initially be put into an actions queue, which you can view with `status`.
For these queued actions, `commit` causes digital changes be reflected in the local file system. On the gui, you will be able to use GPT to help you manage and clean your file system. The current
directory in which you are in serves as the context for the LLM and all AI changes will be for the targeted directory.

Operational Example: https://youtu.be/fxL_ETcNYUM

