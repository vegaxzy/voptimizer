import { ShieldAlert } from "lucide-react";

interface AdminBannerProps {
  onRestartAsAdmin: () => void;
}

export function AdminBanner({ onRestartAsAdmin }: AdminBannerProps) {
  return (
    <div className="admin-banner">
      <span className="admin-banner-icon">
        <ShieldAlert size={14} strokeWidth={2} />
      </span>
      <span className="admin-banner-text">
        Limited mode — some actions require administrator privileges.
      </span>
      <button className="admin-banner-btn" onClick={onRestartAsAdmin}>
        Restart as Admin
      </button>
    </div>
  );
}
