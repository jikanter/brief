# Skill Scaffolding and Validation Demo

*2026-04-13T18:28:57Z by Showboat 0.6.1*
<!-- showboat-id: ec5b2dd3-4e99-44a1-abca-6e2479e778d0 -->

This demo shows how to use `brief` to scaffold a new skill and then validate it against the Agent Skills standard.

```bash
DOC_PATH=/tmp/sample-doc.md && echo -e '# my-skill\nThis is my new skill' > $DOC_PATH && ./target/debug/brief skill scaffold --from-doc $DOC_PATH && mv "#-my-skill" /tmp/my-skill && ls -R /tmp/my-skill
```

```output
Scaffolded skill in "#-my-skill"
references
scripts
SKILL.md

/tmp/my-skill/references:

/tmp/my-skill/scripts:
```

Now we validate the generated skill to ensure it meets the Agent Skills specification.

```bash
./target/debug/brief skill validate /tmp/my-skill && echo 'Validation successful!'
```

```output
SKILL.md at "/tmp/my-skill/SKILL.md" is valid.
Validation successful!
```

Finally, we clean up the temporary directory created during the demo.

```bash
rm -rf /tmp/my-skill /tmp/sample-doc.md && echo 'Cleanup complete!'
```

```output
Cleanup complete!
```
