import { Button, ButtonProps } from "./button";
import { twMerge } from "tailwind-merge";

type GenerateButtonProps = ButtonProps;

export const GenerateButton = ({
  children,
  className,
  disabled,
  ...props
}: GenerateButtonProps) => {
  return (
    <Button
      className={twMerge(
        "group flex items-center justify-center gap-2 h-[34px]",
        className,
      )}
      disabled={disabled}
      {...props}
    >
      <span className="truncate">{children}</span>

    </Button>
  );
};

export default GenerateButton;
