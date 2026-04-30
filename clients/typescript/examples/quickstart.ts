import { MarkdownFS } from "markdownfs";

const mdfs = new MarkdownFS({
  baseUrl: process.env.MDFS_URL ?? "http://127.0.0.1:3000",
  token: process.env.MDFS_TOKEN,
});

await mdfs.fs.write("notes/hello.md", "# Hello from MarkdownFS\n");

console.log(await mdfs.fs.read("notes/hello.md"));
console.log(await mdfs.fs.list("notes"));

const { hash } = await mdfs.vcs.commit("first note");
console.log("committed", hash);
