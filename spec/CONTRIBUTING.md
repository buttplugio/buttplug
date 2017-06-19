# Contributing

Thanks for your interest in contributing to the Metafetish
Organization repo! We're looking forward to working with you to make
this project better.

## Code Of Conduct

First off, you'll want to check out our [Code of Conduct](CODE_OF_CONDUCT.md), which
should be alongside this document. It outlines the rules and
expectations for interaction with our project.

## For All Community

This section contains information for everyone involved with the
projects, whether developing or simply interested in using and
recommending improvements.

### Communication

There are a couple of different ways in which you can interact with
other members of Metafetish projects.

- [We have a Slack instance](https://metafetish.slack.com). Note that an invitation is required
  to join the slack instance. Please email admin@metafetish.com for an
  invite, and include the email address you'd like to use to log in.
- [We have message boards](http://metafetish.club). We have Discourse forums available that
  cover most of our projects. For anything that doesn't fit into one
  of the categories, there's the General forum.

### Anonymous Accounts

Due to the sensitive nature of Metafetish projects, some community
members prefer to use anonymous accounts, on message boards as well as
for contributing to code repos. We understand the need for this, and
try to be as accepting of that as possible without letting it
interfere with project progress. 

Note that vetting by project leads will still need to occur before
administration rights after given to any account on a project resource
(forums, repos, etc), anonymous or otherwise.

### Filing feature requests

If there are features you'd like in a project, you may request them by:

- If you have a github account, filing a github issue on the project.
- Otherwise, make a post on the message board in the appropriate
  category, or on the General category if a proper category does not
  exist.
  
Please be specific in your feature request. We will ask followup
questions for clarification, but the more information we have, the
better.

### Filing bug reports

If you find a problem in a project, please do not hesitate to tell us.

- If you find a security bug, please email [admin@metafetish.com](mailto:admin@metafetish.com)
  immediately, and we will work with you to resolve it and get the
  information out to the community ASAP.
- For all other bugs:
  - If you have a github account, filing a github issue on the project.
  - Otherwise, make a post on the message board in the appropriate
    category, or on the General category if a proper category does not
    exist.

In the issue or post, you should let us know:

- The software you are using that has the bug
- The version of the software
- The operating system version of the computer you using the software
  on.
- The steps you took to get to the problem.
- Whether the problem is repeatable.

Someone should hopefully follow up on your problem soon.

## For Developer Community

This section contains information mainly related to helping in
development of Metafetish projects.

### Getting up and running on a project

In many cases, if you are trying to start developing a new project,
information about compiling and using the project will be in the
README. If these instructions are missing or incomplete, you can ask
in the [proper category on the message boards](   http://metafetish.club), or file an issue on
the project if you have a github account. You can also contact us on
Slack if you have an account there.

Note that some of our projects are rather complicated, and span
multiple repositories and/or technologies. We do our best to keep
things up to date, but there may be times where we've missed updating
documentation. If something seems wrong, or isn't working for you, ask
us using one of the above methods.

### Continuous Integration

In as many cases as possible, we have added continuous integration
services to run build checks on our software projects. These will
normally be [Travis](http://travis-ci.org) for macOS and linux builds (or platform
independent builds, like Node with no native requirements),
and [Appveyor](http://appveyor.com) for Windows builds. CI Badges are usually added to
the README.

### Git/Github Workflow

This section goes over our git workflow. We realize that git can be
quite complicated and has a steep learning curve. We have done as much
as we can to make sure Github makes this easy for contributors. If you
are new to git, or if you do not understand some part of this section,
please let us know when you make a pull request, and we'll help out.
If you are not sure how to make a pull request on github,
contact [admin@metafetish.com](mailto:admin@metafetish.com) and a project lead will help.

As of this writing, Metafetish projects are maintained on
the [Metafetish Organization on Github](http://github.com/metafetish). 'master' branches on
Metafetish projects are kept as [Github protected branches](https://help.github.com/articles/about-protected-branches/), with
the following settings.

- All of the following rules apply to both users and administrators.
- No direct pushes to 'master'. All changes must be via Pull Request
  (PR).
- No force pushes to 'master'. All rewrites must be done on feature
  branches.
- PRs must be off the end of the 'master' branch to merge to master.
  Github will enforce this in PRs.
- PRs must pass CI to merge. Due to the hardware focus of many
  Metafetish projects, tests may be difficult to write in languages
  without proper mocking utilities. Therefore, Code Coverage increase
  is nice, but not required.
- PRs should have a reviewer if possible, but this is not enforced.

Metafetish organization projects maintain a 'rebase-only' workflow to
master when possible, where all branches will be a fast-forward merge.
Github PRs should manage this themselves, and will display an error if
this is not possible. Project management will be happy to work with
you to resolve the issue.

In order to reduce workload of contributors, repo dependencies should
be brought in by using the [git subtree method](https://developer.atlassian.com/blog/2015/05/the-power-of-git-subtree/) instead of git
submodules. As this will require upkeep and documentation, please
discuss possible repo inclusion with project leads before submitting
pull requests with subtree merges.

### Project Management

For project management, we usually use either [Trello](http://trello.com)
or [ZenHub](http://zenhub.io), depending on the level of integration needed with the
source code repo itself. More information about this is usually
included in the README for the specific project.

### Documentation

Non-code documentation for projects is usually done in one of two
formats:

- Markdown, for all README and contributor facing files.
- org-mode, for large documentation sets and manuals.

As there is currently only one project lead using org-mode (but they
write most of the documentation), conversation from org-mode to
markdown can happen on request. Similarly, markdown versions of
org-mode documents may be checked in to documentation repos as needed.

Large manuals are usually managed using the [gitbook](https://github.com/GitbookIO/gitbook) format.
