import Link from 'next/link';
import type { Metadata } from 'next';
import { PathUtils } from 'fumadocs-core/source';
import { blog } from '@/lib/source';

export const metadata: Metadata = {
  title: 'Tesser Blog',
  description:
    'Dispatches for investors and developers building on the Tesser quantitative OS.',
};

function toDateInput(path: string) {
  const filename = PathUtils.basename(path, PathUtils.extname(path));
  const match = filename.match(/\d{4}-\d{2}-\d{2}/);

  return match?.[0] ?? '1970-01-01';
}

export default function BlogIndexPage() {
  const posts = [...blog.getPages()].sort(
    (a, b) =>
      new Date(
        (b.data.date as string | Date | undefined) ?? toDateInput(b.path),
      ).getTime() -
      new Date(
        (a.data.date as string | Date | undefined) ?? toDateInput(a.path),
      ).getTime(),
  );

  return (
    <main className="relative isolate min-h-screen overflow-hidden bg-black text-white">
      <div className="pointer-events-none absolute inset-0 opacity-60">
        <div className="absolute inset-0 bg-gradient-to-b from-blue-600/20 via-black to-black blur-3xl" />
        <div className="absolute right-[-20%] top-[-20%] h-[480px] w-[480px] rounded-full bg-purple-700/30 blur-[140px]" />
      </div>
      <div className="relative mx-auto flex w-full max-w-6xl flex-col gap-16 px-6 py-32">
        <div className="max-w-3xl space-y-6">
          <p className="text-sm uppercase tracking-[0.3em] text-blue-300">
            Dispatch
          </p>
          <h1 className="text-4xl font-bold leading-tight text-white md:text-5xl">
            Insights for long-term partners, investors, and the engineers
            building on Tesser.
          </h1>
          <p className="text-lg text-zinc-400">
            Field notes from the team covering execution architecture, strategy
            research, and the path to a programmable market operating system.
          </p>
        </div>

        <div className="grid grid-cols-1 gap-6 md:grid-cols-2">
          {posts.map((post) => {
            const date = new Date(
              (post.data.date as string | Date | undefined) ??
                toDateInput(post.path),
            );

            return (
              <Link
                key={post.url}
                href={post.url}
                className="group relative flex flex-col rounded-3xl border border-white/10 bg-zinc-900/30 p-6 transition ring-offset-2 hover:-translate-y-1 hover:border-white/30 hover:bg-white/5 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-white/60"
              >
                <span className="text-xs font-mono uppercase tracking-[0.4em] text-blue-300">
                  {date.toLocaleDateString(undefined, {
                    month: 'short',
                    day: 'numeric',
                    year: 'numeric',
                  })}
                </span>
                <h2 className="mt-3 text-2xl font-semibold text-white">
                  {post.data.title}
                </h2>
                <p className="mt-2 flex-1 text-sm leading-relaxed text-zinc-400">
                  {post.data.description}
                </p>
                <span className="mt-6 inline-flex items-center gap-2 text-sm font-semibold text-blue-300">
                  Read Dispatch
                  <svg
                    viewBox="0 0 24 24"
                    className="size-4 transition-transform group-hover:translate-x-1"
                    fill="none"
                    stroke="currentColor"
                    strokeWidth="2"
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    aria-hidden
                  >
                    <path d="M5 12h14" />
                    <path d="m12 5 7 7-7 7" />
                  </svg>
                </span>
              </Link>
            );
          })}
        </div>
      </div>
    </main>
  );
}
