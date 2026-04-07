interface SuiteNavProps {
  current: 'ip' | 'dns' | 'tls';
}

const LINKS = [
  { id: 'ip',  label: 'IP',  href: 'https://ip.netray.info' },
  { id: 'dns', label: 'DNS', href: 'https://dns.netray.info' },
  { id: 'tls', label: 'TLS', href: 'https://tls.netray.info' },
] as const;

export default function SuiteNav(props: SuiteNavProps) {
  return (
    <>
      <style>{`
        .suite-nav {
          width: 100%;
          background: var(--bg-secondary);
          border-bottom: 1px solid var(--border-subtle);
          padding: 0.5rem 1.5rem;
          display: flex;
          align-items: center;
          gap: 0;
          font-family: var(--font-mono);
          font-size: 0.8rem;
          flex-wrap: wrap;
          row-gap: 0.25rem;
        }

        .suite-nav__brand {
          color: var(--text-secondary);
          text-decoration: none;
          padding: 0.2rem 0.5rem 0.2rem 0;
          letter-spacing: -0.01em;
          white-space: nowrap;
          min-height: 44px;
          display: inline-flex;
          align-items: center;
        }

        .suite-nav__brand:hover {
          color: var(--text-primary);
          text-decoration: none;
        }

        .suite-nav__sep {
          color: var(--border);
          padding: 0 0.5rem;
          user-select: none;
        }

        .suite-nav__link {
          color: var(--text-secondary);
          text-decoration: none;
          padding: 0.2rem 0.4rem;
          border-radius: 4px;
          white-space: nowrap;
          transition: color 0.15s ease;
          min-height: 44px;
          display: inline-flex;
          align-items: center;
        }

        .suite-nav__link:hover {
          color: var(--text-primary);
          text-decoration: none;
        }

        .suite-nav__link--active {
          color: var(--accent);
          font-weight: 700;
          border-bottom: 2px solid var(--accent);
          border-radius: 0;
          padding-bottom: calc(0.2rem - 2px);
        }

        .suite-nav__link--active:hover {
          color: var(--accent);
        }

        @media (max-width: 400px) {
          .suite-nav {
            padding: 0.4rem 1rem;
            font-size: 0.75rem;
          }
        }
      `}</style>
      <nav class="suite-nav" aria-label="netray.info suite">
        <a class="suite-nav__brand" href="https://netray.info">netray.info</a>
        {LINKS.map((link) => (
          <>
            <span class="suite-nav__sep" aria-hidden="true">|</span>
            <a
              class={`suite-nav__link${props.current === link.id ? ' suite-nav__link--active' : ''}`}
              href={link.href}
              aria-current={props.current === link.id ? 'page' : undefined}
            >
              {link.label}
            </a>
          </>
        ))}
      </nav>
    </>
  );
}
