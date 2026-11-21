# Planned issues

Drafts of GitHub issues. Each `NN-name.md` file is one issue: the first line (`# Title`) is the
title, the rest is the body (a detailed description: what, why, how to verify). Create them with

```
scripts\create-issues.ps1 -DryRun     # shows what would be created
scripts\create-issues.ps1             # creates the missing ones (existing titles are skipped)
```

Needs the GitHub CLI (`gh`) signed in. Files without the `NN-` prefix (like this one) are ignored.
Set the repository with `-Repo MiroM73/<Name>` – the default in the script is the template's.
