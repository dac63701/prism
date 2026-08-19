"use client";

interface Section {
  id: string;
  title: string;
  items: { id: string; heading: string; href?: string }[];
}

export function MobileDocsNav({ sections }: { sections: Section[] }) {
  return (
    <div className="fixed bottom-0 left-0 right-0 z-30 border-t border-border bg-[#050816]/95 p-3 lg:hidden">
      <select
        className="w-full rounded-xl border border-border bg-surface px-4 py-3 text-sm text-white"
        onChange={(e) => {
          const option = e.target.selectedOptions[0];
          const href = option?.dataset.href;
          if (href) {
            window.location.href = href;
            return;
          }
          const el = document.getElementById(e.target.value);
          if (el) el.scrollIntoView({ behavior: "smooth" });
        }}
      >
        {sections.map((section) => (
          <optgroup key={section.id} label={section.title}>
            {section.items.map((item) => (
              <option key={item.id} value={item.id} data-href={item.href}>
                {item.heading}
              </option>
            ))}
          </optgroup>
        ))}
      </select>
    </div>
  );
}
