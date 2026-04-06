import type { Metadata } from "next";
import "./globals.css";

export const metadata: Metadata = {
  title: "howfastwould",
  description: "how fast would AI solve this leetcode problem?",
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en">
      <body className="min-h-screen" style={{ background: "var(--bg)" }}>
        {children}
      </body>
    </html>
  );
}
