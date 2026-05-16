type SearchBarProps = {
  placeholder: string;
  value: string;
  onChange: (value: string) => void;
};

export function SearchBar({ placeholder, value, onChange }: SearchBarProps) {
  return (
    <div className="add-item-search">
      <span className="add-item-search-icon" aria-hidden="true">
        🔍
      </span>
      <input
        className="add-item-search-input"
        type="text"
        placeholder={placeholder}
        value={value}
        onChange={(e) => onChange(e.target.value)}
      />
      {value && (
        <button
          className="add-item-search-clear"
          onClick={() => onChange("")}
          aria-label="Clear search"
        >
          ✕
        </button>
      )}
    </div>
  );
}
