import { HtmlBasePlugin } from "@11ty/eleventy";
import syntaxHighlight from "@11ty/eleventy-plugin-syntaxhighlight";
import markdownIt from "markdown-it";
import markdownItAnchor from "markdown-it-anchor";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { FaGithub, FaXTwitter } from "react-icons/fa6";

const icons = { github: FaGithub, x: FaXTwitter };

const slugify = (text) =>
  text
    .trim()
    .toLowerCase()
    .replace(/[^\p{L}\p{N}\s-]/gu, "")
    .replace(/\s+/g, "-");

const pageHref = (href) => {
  const m = href.match(/^(?:\.\.\/)?([\w-]+)\.md(#.*)?$/i);
  if (!m) return href;
  const [, name, hash = ""] = m;
  if (name === "index") return `/${hash}`;
  return `/${name.toLowerCase()}/${hash}`;
};

export default function (eleventyConfig) {
  const md = markdownIt({ html: true, linkify: true }).use(markdownItAnchor, {
    level: [2, 3],
    slugify,
    tabIndex: false,
  });

  const defaultLink = md.renderer.rules.link_open || ((tokens, i, opts, env, self) => self.renderToken(tokens, i, opts));
  md.renderer.rules.link_open = (tokens, i, opts, env, self) => {
    const href = tokens[i].attrGet("href");
    if (href) tokens[i].attrSet("href", pageHref(href));
    if (/^https?:\/\//.test(href || "")) tokens[i].attrSet("rel", "noopener");
    return defaultLink(tokens, i, opts, env, self);
  };

  md.renderer.rules.table_open = () => '<div class="table-wrap"><table>';
  md.renderer.rules.table_close = () => "</table></div>";

  eleventyConfig.setLibrary("md", md);
  eleventyConfig.addPlugin(syntaxHighlight);
  eleventyConfig.addPlugin(HtmlBasePlugin);
  eleventyConfig.addPassthroughCopy("assets");
  eleventyConfig.addWatchTarget("assets");

  eleventyConfig.addCollection("docs", (api) =>
    api.getFilteredByTag("docs").sort((a, b) => a.data.order - b.data.order),
  );

  eleventyConfig.addFilter("markdown", (text) => md.render(text || ""));

  eleventyConfig.addShortcode("icon", (name, size = 18) =>
    renderToStaticMarkup(createElement(icons[name], { size, "aria-hidden": true, focusable: false })),
  );

  eleventyConfig.addFilter("outline", (html) => {
    const heads = [];
    const re = /<h([23]) id="([^"]+)"[^>]*>([\s\S]*?)<\/h\1>/g;
    let m;
    while ((m = re.exec(html || ""))) {
      heads.push({ level: Number(m[1]), id: m[2], text: m[3].replace(/<[^>]+>/g, "") });
    }
    return heads;
  });

  eleventyConfig.addFilter("fromFirstSection", (html) => {
    const i = (html || "").search(/<h2\b/);
    return i === -1 ? html : html.slice(i);
  });

  return {
    dir: { input: ".", output: "_site", includes: "_includes", data: "_data" },
    pathPrefix: "/mido/",
    markdownTemplateEngine: false,
    htmlTemplateEngine: "njk",
  };
}
