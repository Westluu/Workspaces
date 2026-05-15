type ErrorToastProps = {
  error: string;
};

export function ErrorToast({ error }: ErrorToastProps) {
  return <span className="error-toast">{error}</span>;
}
