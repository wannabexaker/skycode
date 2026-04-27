import React, { createContext, useContext, useState, useCallback } from "react";

interface ConfirmDialogState {
  isOpen: boolean;
  title: string;
  message: string;
  confirmText: string;
  cancelText: string;
  onConfirm: () => void;
  onCancel: () => void;
  isDangerous: boolean;
}

interface AlertDialogState {
  isOpen: boolean;
  title: string;
  message: string;
  onClose: () => void;
}

interface PromptDialogState {
  isOpen: boolean;
  title: string;
  message: string;
  defaultValue: string;
  onConfirm: (value: string) => void;
  onCancel: () => void;
  placeholder?: string;
}

interface DialogContextType {
  showConfirm: (options: {
    title?: string;
    message: string;
    confirmText?: string;
    cancelText?: string;
    isDangerous?: boolean;
  }) => Promise<boolean>;
  showAlert: (options: { title?: string; message: string }) => Promise<void>;
  showPrompt: (options: {
    title?: string;
    message: string;
    defaultValue?: string;
    placeholder?: string;
  }) => Promise<string | null>;
}

const DialogContext = createContext<DialogContextType | null>(null);

export const useDialogs = () => {
  const context = useContext(DialogContext);
  if (!context) {
    throw new Error("useDialogs must be used within DialogProvider");
  }
  return context;
};

export const DialogProvider: React.FC<{ children: React.ReactNode }> = ({
  children,
}) => {
  const [confirmState, setConfirmState] = useState<ConfirmDialogState>({
    isOpen: false,
    title: "",
    message: "",
    confirmText: "Confirm",
    cancelText: "Cancel",
    onConfirm: () => {},
    onCancel: () => {},
    isDangerous: false,
  });

  const [alertState, setAlertState] = useState<AlertDialogState>({
    isOpen: false,
    title: "",
    message: "",
    onClose: () => {},
  });

  const [promptState, setPromptState] = useState<PromptDialogState>({
    isOpen: false,
    title: "",
    message: "",
    defaultValue: "",
    onConfirm: () => {},
    onCancel: () => {},
    placeholder: "",
  });

  const [promptInput, setPromptInput] = useState("");

  const showConfirm = useCallback((options: {
    title?: string;
    message: string;
    confirmText?: string;
    cancelText?: string;
    isDangerous?: boolean;
  }): Promise<boolean> => {
    return new Promise((resolve) => {
      setConfirmState({
        isOpen: true,
        title: options.title || "Confirm",
        message: options.message,
        confirmText: options.confirmText || "Confirm",
        cancelText: options.cancelText || "Cancel",
        isDangerous: options.isDangerous || false,
        onConfirm: () => {
          setConfirmState(prev => ({ ...prev, isOpen: false }));
          resolve(true);
        },
        onCancel: () => {
          setConfirmState(prev => ({ ...prev, isOpen: false }));
          resolve(false);
        },
      });
    });
  }, []);

  const showAlert = useCallback((options: {
    title?: string;
    message: string;
  }): Promise<void> => {
    return new Promise((resolve) => {
      setAlertState({
        isOpen: true,
        title: options.title || "Alert",
        message: options.message,
        onClose: () => {
          setAlertState(prev => ({ ...prev, isOpen: false }));
          resolve();
        },
      });
    });
  }, []);

  const showPrompt = useCallback((options: {
    title?: string;
    message: string;
    defaultValue?: string;
    placeholder?: string;
  }): Promise<string | null> => {
    return new Promise((resolve) => {
      setPromptInput(options.defaultValue || "");
      setPromptState({
        isOpen: true,
        title: options.title || "Input",
        message: options.message,
        defaultValue: options.defaultValue || "",
        placeholder: options.placeholder || "",
        onConfirm: (value: string) => {
          setPromptState(prev => ({ ...prev, isOpen: false }));
          resolve(value);
        },
        onCancel: () => {
          setPromptState(prev => ({ ...prev, isOpen: false }));
          resolve(null);
        },
      });
    });
  }, []);

  return (
    <DialogContext.Provider value={{ showConfirm, showAlert, showPrompt }}>
      {children}
      <ConfirmDialog {...confirmState} />
      <AlertDialog {...alertState} />
      <PromptDialog
        {...promptState}
        value={promptInput}
        onChange={setPromptInput}
      />
    </DialogContext.Provider>
  );
};

// Shared overlay + box styles
const overlayStyle: React.CSSProperties = {
  position: 'fixed', top: 0, left: 0, right: 0, bottom: 0,
  zIndex: 9999,
  display: 'flex', alignItems: 'center', justifyContent: 'center',
  backgroundColor: 'rgba(0,0,0,0.55)',
  backdropFilter: 'blur(4px)',
};
const boxStyle: React.CSSProperties = {
  width: '100%', maxWidth: '440px', margin: '0 20px',
  borderRadius: '10px',
  border: '1px solid var(--border)',
  backgroundColor: 'var(--surface)',
  boxShadow: '0 24px 48px rgba(0,0,0,0.4)',
  overflow: 'hidden',
};
const headerStyle: React.CSSProperties = {
  display: 'flex', alignItems: 'flex-start', justifyContent: 'space-between',
  padding: '20px 24px',
  borderBottom: '1px solid var(--border)',
};
const bodyStyle: React.CSSProperties = { padding: '20px 24px' };
const footerStyle: React.CSSProperties = {
  display: 'flex', gap: '10px', justifyContent: 'flex-end',
  padding: '16px 24px',
  borderTop: '1px solid var(--border)',
  backgroundColor: 'var(--card)',
};
const closeBtnStyle: React.CSSProperties = {
  background: 'none', border: 'none', cursor: 'pointer',
  fontSize: '18px', padding: '0 0 0 8px', color: 'var(--text-mute)', lineHeight: 1,
};
const secondaryBtnStyle: React.CSSProperties = {
  padding: '8px 16px', borderRadius: '6px', fontSize: '14px', fontWeight: 500,
  backgroundColor: 'transparent', border: '1px solid var(--border)',
  color: 'var(--text)', cursor: 'pointer',
};
const primaryBtnStyle = (danger: boolean): React.CSSProperties => ({
  padding: '8px 18px', borderRadius: '6px', fontSize: '14px', fontWeight: 600,
  backgroundColor: danger ? '#ef4444' : 'var(--accent)',
  border: 'none', color: '#fff', cursor: 'pointer',
});

// Confirm Dialog Component
const ConfirmDialog: React.FC<ConfirmDialogState> = ({
  isOpen, title, message, confirmText, cancelText, onConfirm, onCancel, isDangerous,
}) => {
  if (!isOpen) return null;
  return (
    <div style={overlayStyle} onMouseDown={e => e.target === e.currentTarget && onCancel()}>
      <div style={boxStyle}>
        <div style={headerStyle}>
          <h2 style={{ margin: 0, fontSize: '15px', fontWeight: 600, color: 'var(--text)' }}>{title}</h2>
          <button style={closeBtnStyle} onClick={onCancel}>✕</button>
        </div>
        <div style={bodyStyle}>
          <p style={{ margin: 0, fontSize: '13px', color: 'var(--text-dim)', whiteSpace: 'pre-wrap', lineHeight: '1.5' }}>{message}</p>
        </div>
        <div style={footerStyle}>
          <button style={secondaryBtnStyle} onClick={onCancel}>{cancelText}</button>
          <button style={primaryBtnStyle(isDangerous)} onClick={onConfirm}>{confirmText}</button>
        </div>
      </div>
    </div>
  );
};

// Alert Dialog Component
const AlertDialog: React.FC<AlertDialogState> = ({
  isOpen, title, message, onClose,
}) => {
  if (!isOpen) return null;
  return (
    <div style={overlayStyle} onMouseDown={e => e.target === e.currentTarget && onClose()}>
      <div style={boxStyle}>
        <div style={headerStyle}>
          <h2 style={{ margin: 0, fontSize: '15px', fontWeight: 600, color: 'var(--text)' }}>{title}</h2>
          <button style={closeBtnStyle} onClick={onClose}>✕</button>
        </div>
        <div style={bodyStyle}>
          <p style={{ margin: 0, fontSize: '13px', color: 'var(--text-dim)', whiteSpace: 'pre-wrap', lineHeight: '1.5' }}>
            {message}
          </p>
        </div>
        <div style={footerStyle}>
          <button style={primaryBtnStyle(false)} onClick={onClose}>OK</button>
        </div>
      </div>
    </div>
  );
};

// Prompt Dialog Component
interface PromptDialogProps extends PromptDialogState {
  value: string;
  onChange: (value: string) => void;
}

const PromptDialog: React.FC<PromptDialogProps> = ({
  isOpen,
  title,
  message,
  defaultValue,
  placeholder,
  onConfirm,
  onCancel,
  value,
  onChange,
}) => {
  if (!isOpen) return null;

  const handleConfirm = () => {
    onConfirm(value);
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === "Enter") {
      handleConfirm();
    } else if (e.key === "Escape") {
      onCancel();
    }
  };

  return (
    <div style={overlayStyle} onMouseDown={e => e.target === e.currentTarget && onCancel()}>
      <div style={boxStyle}>
        <div style={headerStyle}>
          <h2 style={{ margin: 0, fontSize: '15px', fontWeight: 600, color: 'var(--text)' }}>{title}</h2>
          <button style={closeBtnStyle} onClick={onCancel}>✕</button>
        </div>
        <div style={bodyStyle}>
          <p style={{ margin: 0, marginBottom: '12px', fontSize: '13px', color: 'var(--text-dim)' }}>{message}</p>
          <input
            type="text"
            value={value}
            onChange={(e) => onChange(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder={placeholder || defaultValue}
            autoFocus
            style={{
              width: '100%', padding: '8px 12px', borderRadius: '6px',
              border: '1px solid var(--border)', backgroundColor: 'var(--card)',
              color: 'var(--text)', fontSize: '14px', boxSizing: 'border-box',
              outline: 'none',
            }}
            onFocus={(e) => { e.currentTarget.style.borderColor = 'var(--accent)'; }}
            onBlur={(e) => { e.currentTarget.style.borderColor = 'var(--border)'; }}
          />
        </div>
        <div style={footerStyle}>
          <button style={secondaryBtnStyle} onClick={onCancel}>Cancel</button>
          <button style={primaryBtnStyle(false)} onClick={handleConfirm}>OK</button>
        </div>
      </div>
    </div>
  );
};
