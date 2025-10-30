development guidelines
======================

## philosophy

1. Just do it.
2. Tests are awesome.
3. Workflow automation is awesome.
4. Don't push crazy stuff to `origin master`

## development process

### manifesto
1. Don't solve solved problems. Use open source solutions when possible.
2. Be focused. Ask yourself if what you're doing helps you finish your current task. If not then don't do it, backlog it.
3. Have fun or go home. This isn't a job.

### productivity
Let's keep track of stuff with [Trello](https://trello.com/b/rDxl2nlC/caverns).

### editor
Don't care what editor you use, but `.gitignore` its stupid files.

### code
Javascript for client and server code. Ruby for workflow scripts.

### workflow automation

#### embrace

* Ruby is sweet.
* Rake is sweet.

#### avoid

Avoid using bash, python, and other friends. Ruby only, pls.

#### rake

* Rake tasks should be namespaced.
* Define tasks in a `.rake` file in `./tasks/`, i.e.: `./tasks/my-sweet-tasks.rake`.
* `import` your `.rake` files in `./rakefile`
* Only `desc` user-facing tasks.

### git smart

Let's be smart about using git.

1. Until we establish a test-driven workflow, **keep `origin master` in working order**.
2. External code should be added to the project as a submodule.

#### branch names

Name your branches in the following way:

##### work in progress
Sometimes you need to push something that isn't ready or maybe doesn't even build.
    
    wip/feature-name
    
##### debug
Sometimes you'll be in the middle of debugging something and need to switch contexts or let someone else take over from where you are. Don't use a `wip/` branch name. Instead, call your branch:

    debug/bug-description

##### experiments
Sometimes you'll have a sweet idea and want to try something out. Do it! And push that bad boy to:

    jake/my-sweet-idea
    
Obviously, replace `jake` with your name, like `jack`, or `arthur`, or your github alias, if you prefer, like `flyingrobots/amazing-thing`.

#### tags
For now, let's tag

* Milestone commits

Perhaps consider tagging these sort of commits in the future…

* Deployments (to track what's in production)
* Automated commits (to identify commits made by tools/scripts)

## milestone names
It'd be sweet/fun to name milestones in the roadmap after national capitals, in alphabetical order. For example:

1. Athens
2. Baghdad
3. Copenhagen
4. Doha
5. Freetown (no E?)
6. Gibraltar
7. Hanoi
8. Islamabad
9. Juba
10. Kiev

And so on. Yea or nay?
