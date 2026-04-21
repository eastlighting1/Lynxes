# Guides

The guides section is where Lynxes should feel teachable.
These pages are not trying to explain the architecture in the abstract, and they are not trying to enumerate every method on the public surface. The goal here is narrower and more practical: help a new user get from zero to one successful result without forcing them to guess what step comes next or whether their output is correct.

If the concepts section answers "why is this engine designed this way?", the guides section should answer "what do I do first, and what should I see when it works?" That difference matters. A guide is not just a short explanation page with a few code blocks. It should lead the user through a small, controlled path where the chance of failure is low and the success signal is obvious.

For that reason, the recommended reading order here is intentional rather than alphabetical.

## Recommended Path

Start here if you are using Python:

1. [Verify Your Install](verify-your-install.md)
2. [Getting Started in Python](getting-started-python.md)
3. [Your First Graph Query](first-graph-query.md)
4. [Your First Algorithm Run](first-algorithm-run.md)

Start here if you want to begin from the CLI:

1. [Getting Started on the CLI](getting-started-cli.md)
2. [Your First Graph Query](first-graph-query.md)

## What Belongs Here

The documents in this section are meant to be learning-oriented. That means they should do a few things consistently:

- begin from a clear starting point
- give the user one happy path instead of many competing options
- show expected output or obvious success checks
- avoid sending the reader off into side topics too early

If a page is mainly about troubleshooting, API signatures, or one-off recipes, it probably belongs somewhere else. Guides should teach; they should not try to be a backup reference or a dumping ground for every useful note.

## What To Read Afterward

Once you have walked through the beginner path, the next stop depends on what you are trying to do.

If you want deeper explanation of the engine's layout or execution model, go to the concepts section.
If you want exact API behavior, parameter names, or file-format details, use the reference pages.
If you already know what task you are trying to accomplish and just want a focused recipe, the cookbook is usually the better fit.
