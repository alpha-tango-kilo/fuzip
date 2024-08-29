# `fuzip` - fuzzy zipping for scripting

Trying to match separate collections of stuff together for zipping

For example, say you have a folder of audio tracks, and a folder of video tracks. They follow a similar, but not identical naming scheme.
If there are 50 files in each and you want to combine the corresponding pairs with ffmpeg or something, you do not want to do this by hand, but also this really doesn't feel like something you should have to crack out a high level scripting langauge for.
That's where `fuzip` wants to come in - you'd point it at both the two folders and it'd output the most likely pairs of of files.
I'd also plan on having some kind of command execution support with templating, similar to `xargs` and `find`.

## Proof of concept

Bipartite maximal matching - let's not try and zip N collections as apparently that's hell

Command execution with templating

## Beyond proof of concept

Maybe try N collection zipping

Customisable handling of incomplete matching (e.g. if one folder has ten files and the other has seven)

Abstraction over inputs: support reading from files instead of using directory listings etc.
