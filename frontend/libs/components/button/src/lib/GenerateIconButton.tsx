import { ArrowUpIcon, LoaderCircleIcon } from "lucide-react";
import { DynamicIcon } from "@storyteller/icons";
import { ButtonHTMLAttributes } from "react";
import { twMerge } from "tailwind-merge";

interface GenerateIconButtonProps extends Omit<
  ButtonHTMLAttributes<HTMLButtonElement>,
  "children"
> {
  loading?: boolean;
}

/**
 * Compact generate control: credit cost as plain text next to a circular
 * arrow-up button. Desktop promptboxes use this; the mobile form keeps the
 * labeled `GenerateButton` pill.
 */
export const GenerateIconButton = ({
  loading,
  className,
  disabled,
  ...rest
}: GenerateIconButtonProps) => {
  const isDisabled = disabled || loading;

  return (
    <div className={twMerge("flex shrink-0 items-center gap-2.5", className)}>
      <button
        type="button"
        className="flex h-9 w-9 items-center justify-center rounded-full bg-primary text-white shadow-sm transition-all duration-150 hover:bg-primary-400 active:scale-95 disabled:cursor-not-allowed disabled:opacity-40 disabled:active:scale-100"
        disabled={isDisabled}
        {...rest}
      >
        <DynamicIcon
          icon={loading ? LoaderCircleIcon : ArrowUpIcon}
          className={loading ? "animate-spin" : undefined}
        />
      </button>
    </div>
  );
};

export default GenerateIconButton;
