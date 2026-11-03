import { useEffect } from "react";
import { create } from "zustand";
import { XIcon } from "lucide-react";
import { DynamicIcon } from "@storyteller/icons";
import { twMerge } from "tailwind-merge";
import { useGenerateApps, useEditApps, getBadgeStyles, goToApp, type FullAppItem } from "~/config/appMenu";

interface AppsDrawerState {
  open: boolean;
  setOpen: (open: boolean) => void;
  toggle: () => void;
}

export const useAppsDrawer = create<AppsDrawerState>((set) => ({
  open: false,
  setOpen: (open) => set({ open }),
  toggle: () => set((s) => ({ open: !s.open })),
}));

/** The tools, as rounded boxes in a panel that slides in from the left. Picking one opens it and closes the panel. */
export const AppsDrawer = () => {
  const { open, setOpen } = useAppsDrawer();
  const generateApps = useGenerateApps();
  const editApps = useEditApps();

  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && setOpen(false);
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [open, setOpen]);

  const pick = (app: FullAppItem) => {
    goToApp(app.action);
    setOpen(false);
  };

  return (
    <>
      <div
        aria-hidden
        className={twMerge(
          "fixed inset-0 z-40 bg-black/30 transition-opacity duration-200",
          open ? "opacity-100" : "pointer-events-none opacity-0",
        )}
        onClick={() => setOpen(false)}
      />
      <aside
        aria-label="Tools"
        aria-hidden={!open}
        className={twMerge(
          "fixed bottom-0 left-0 top-0 z-50 flex w-[340px] max-w-[90vw] flex-col gap-6 overflow-y-auto border-r border-ui-border bg-ui-background p-5 pt-[72px] text-base-fg shadow-xl transition-transform duration-200",
          open ? "translate-x-0" : "-translate-x-full",
        )}
      >
        <button
          aria-label="Close tools"
          className="absolute right-4 top-[68px] rounded-full p-1.5 text-base-fg/60 hover:bg-ui-controls hover:text-base-fg"
          onClick={() => setOpen(false)}
        >
          <XIcon size={18} />
        </button>
        <Section title="Create" apps={generateApps} open={open} onPick={pick} />
        <Section title="Edit" apps={editApps} open={open} onPick={pick} />
      </aside>
    </>
  );
};

const Section = ({ title, apps, open, onPick }: { title: string; apps: FullAppItem[]; open: boolean; onPick: (app: FullAppItem) => void }) => (
  <section>
    <h2 className="mb-3 text-xs font-medium uppercase tracking-wider text-ui-accent-ink">{title}</h2>
    <div className="grid grid-cols-3 gap-3">
      {apps.map((app) => (
        <button
          key={app.id}
          tabIndex={open ? 0 : -1}
          title={app.description}
          disabled={!app.action}
          onClick={() => onPick(app)}
          className="group flex flex-col items-center gap-1.5 rounded-2xl p-1 text-center disabled:opacity-50"
        >
          <span className="relative flex h-16 w-16 items-center justify-center rounded-2xl border border-ui-controls-border bg-ui-controls text-ui-accent-ink transition-colors group-hover:border-primary group-hover:bg-primary/10">
            <DynamicIcon icon={app.icon} className="text-2xl" />
            {app.badge && (
              <span className={twMerge("absolute -right-1 -top-1 rounded-full border bg-ui-background px-1.5 py-0.5 text-[8px] font-semibold uppercase leading-none", getBadgeStyles(app.badge))}>
                {app.badge}
              </span>
            )}
          </span>
          <span className="text-xs font-medium leading-tight">{app.label}</span>
        </button>
      ))}
    </div>
  </section>
);
