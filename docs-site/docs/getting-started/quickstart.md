# Quickstart

Boot the server, write a file, commit it, read it back.

## 1. Start the server

```bash
docker run -p 7860:7860 -e MARKDOWNFS_LISTEN=0.0.0.0:7860 \
  ghcr.io/subramanya1997/markdownfs:latest
```

## 2. Write your first file

=== "TypeScript"

    ```ts
    import { MarkdownFS } from "markdownfs";

    const mdfs = new MarkdownFS({ baseUrl: "http://localhost:7860" });

    await mdfs.fs.write("notes/idea.md", "# my idea\n");
    console.log(await mdfs.fs.read("notes/idea.md"));
    ```

=== "Python"

    ```python
    from markdownfs import MarkdownFS

    mdfs = MarkdownFS(base_url="http://localhost:7860")

    mdfs.fs.write("notes/idea.md", "# my idea\n")
    print(mdfs.fs.read("notes/idea.md"))
    ```

=== "curl"

    ```bash
    curl -X PUT http://localhost:7860/fs/notes/idea.md \
      --data-binary "# my idea"
    curl http://localhost:7860/fs/notes/idea.md
    ```

## 3. Commit it

=== "TypeScript"

    ```ts
    const { hash } = await mdfs.vcs.commit("first note");
    const { commits } = await mdfs.vcs.log();
    console.log(commits);
    ```

=== "Python"

    ```python
    commit = mdfs.vcs.commit("first note")
    print(mdfs.vcs.log())
    ```

## 4. Search across files

=== "TypeScript"

    ```ts
    const hits = await mdfs.search.grep("idea", { recursive: true });
    ```

=== "Python"

    ```python
    hits = mdfs.search.grep("idea", recursive=True)
    ```

## Next

- [Concepts: virtual filesystem →](../concepts/filesystem.md)
- [Deploy on Hugging Face →](huggingface.md)
- [Use as agent memory via MCP →](../api/mcp.md)
