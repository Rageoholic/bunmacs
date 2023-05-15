#Bunmacs! 

An editor inspired by Emacs! 

##Motivation 

I was an avid emacs user. I learned it in 2014 and used it up until 2021 and I
mostly loved it. Sure it's a little jank but it's ancient software and is so
configurable that you can make whatever the hell you need. I was getting wrist
pain so I cobbled together a modal editing scheme that would feel natural to me.
It was awesome. Buuuut emacs is to a degree constrained by its legacy. Emacs
Lisp is so jank that it *parses comments* to support lexical scoping instead of
dynamic scoping. [Even RMS, noted curmudgeon he is, admits that the issues with
emacs are deeper than the surface
level](https://lists.gnu.org/archive/html/emacs-devel/2020-04/msg00885.html).
Once your compiler/interpreter is parsing comments you know that things have
gotten a bit out of hand. Plus there are lots of legacy performance issues,
especially on windows  that I don't want to spend time hunting down and
optimizing. Emacs has some tools for it but it's not great. And things like GUD
(the debugger integration) are yet another example of things feeling worse than
they should. So instead of trying to fix my Emacs config I decided to make my
own editor, figuring that this could at the very worst be a learning experience
for why we don't do things.

##TL;DR 

Emacs is great but also it sucks and I want something a bit more modern