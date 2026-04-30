import os

from markdownfs import MarkdownFS

mdfs = MarkdownFS(
    base_url=os.environ.get("MDFS_URL", "http://127.0.0.1:3000"),
    token=os.environ.get("MDFS_TOKEN"),
)

mdfs.fs.write("notes/hello.md", "# Hello from MarkdownFS\n")

print(mdfs.fs.read("notes/hello.md"))
print(mdfs.fs.list("notes"))

result = mdfs.vcs.commit("first note")
print("committed", result["hash"])
