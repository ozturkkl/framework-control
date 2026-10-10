import { readable } from 'svelte/store';

// Served from the main branch so the announcement can change without a release.
const FEED_URL =
	(import.meta.env?.VITE_ANNOUNCEMENT_URL as string | undefined) ||
	'https://raw.githubusercontent.com/ozturkkl/framework-control/main/ANNOUNCEMENT.md';

const IFRAME_SRC = /^https:\/\/(www\.)?(youtube\.com|youtube-nocookie\.com)\/embed\//;

const FRONTMATTER = /^---\r?\n([\s\S]*?)\r?\n---\r?\n/;
const DATE_LINE = /^date:\s*(\S+)\s*$/m;

/** `date` is an ISO date (or datetime) compared as a string against the last dismissed one; `html` is sanitized. */
export type Announcement = { date: string; html: string };

/** Fetched and rendered once; stays null when the feed is empty or unavailable. */
export const announcement = readable<Announcement | null>(null, (set) => {
	loadAnnouncement()
		.then(set)
		.catch((e) => console.warn('[announcement] unavailable', e));
});

async function loadAnnouncement(): Promise<Announcement | null> {
	const res = await fetch(FEED_URL, { cache: 'no-store' });
	if (!res.ok) throw new Error(`HTTP ${res.status}`);
	const text = await res.text();
	if (!text.trim()) return null;
	const frontmatter = text.match(FRONTMATTER);
	const date = frontmatter?.[1].match(DATE_LINE)?.[1];
	if (!frontmatter || !date) throw new Error('missing `date:` frontmatter');
	return { date, html: await render(text.slice(frontmatter[0].length)) };
}

// Loaded lazily so the parser and sanitizer stay out of the main bundle.
async function render(markdown: string): Promise<string> {
	const [{ marked }, { default: DOMPurify }] = await Promise.all([import('marked'), import('dompurify')]);
	DOMPurify.addHook('uponSanitizeAttribute', (node, data) => {
		if (node.tagName === 'IFRAME' && data.attrName === 'src' && !IFRAME_SRC.test(data.attrValue)) {
			data.keepAttr = false;
		}
	});
	DOMPurify.addHook('afterSanitizeAttributes', (node) => {
		if (node.tagName === 'A') {
			node.setAttribute('target', '_blank');
			node.setAttribute('rel', 'noopener noreferrer');
		}
	});
	const html = await marked.parse(markdown, { gfm: true });
	return DOMPurify.sanitize(html, {
		ADD_TAGS: ['iframe'],
		ADD_ATTR: ['allow', 'allowfullscreen'],
	});
}
