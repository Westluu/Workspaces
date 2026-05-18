import { useRef } from "react";

type SearchBarProps = {
  placeholder: string;
  value: string;
  onChange: (value: string) => void;
};

export function SearchBar({ placeholder, value, onChange }: SearchBarProps) {
  const inputRef = useRef<HTMLInputElement>(null);

  return (
    <div
      className="add-item-search"
      role="search"
      onMouseDown={(event) => {
        if (event.target instanceof HTMLButtonElement) {
          return;
        }

        inputRef.current?.focus();
      }}
    >
      <span className="add-item-search-icon" aria-hidden="true">
        🔍
      </span>
      <input
        ref={inputRef}
        className="add-item-search-input"
        type="search"
        placeholder={placeholder}
        value={value}
        onChange={(event) => onChange(event.currentTarget.value)}
      />
      {value && (
        <button
          className="add-item-search-clear"
          type="button"
          onClick={() => {
            onChange("");
            inputRef.current?.focus();
          }}
          aria-label="Clear search"
        >
          ✕
        </button>
      )}
    </div>
  );
}
