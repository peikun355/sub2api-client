import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { Sub2APIQuickStart } from "@/components/sub2api/Sub2APIQuickStart";

const invokeMock = vi.fn();
let setupListener: ((event: { payload: { ticket: string } }) => void) | undefined;

vi.mock("@tauri-apps/api/core", () => ({ invoke: (...args: unknown[]) => invokeMock(...args) }));
vi.mock("@tauri-apps/api/app", () => ({ getVersion: () => Promise.resolve("0.1.0") }));
vi.mock("@tauri-apps/api/event", () => ({
  listen: (_event: string, listener: typeof setupListener) => {
    setupListener = listener;
    return Promise.resolve(() => undefined);
  },
}));

const preview = {
  site_name: "Sub2API",
  api_base_url: "https://ai.tavonilo.com",
  model: "gpt-5.6-terra",
  reasoning_effort: "high",
  wire_api: "responses",
};

describe("Sub2APIQuickStart", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    setupListener = undefined;
    invokeMock.mockImplementation((command: string) => {
      if (command === "sub2api_setup_status") return Promise.resolve(false);
      if (command === "sub2api_take_queued_ticket") return Promise.resolve(null);
      if (command === "sub2api_prepare_manual" || command === "sub2api_prepare_ticket") return Promise.resolve(preview);
      if (command === "sub2api_apply_setup") return Promise.resolve({ configured: true, verified: true, server_confirmed: true, backup_directory: "/tmp/backup", restart_required: true });
      return Promise.reject(new Error(`unexpected command ${command}`));
    });
  });

  it("previews manual configuration and requires confirmation before apply", async () => {
    const { container } = render(<Sub2APIQuickStart onAdvanced={() => undefined} />);
    const input = container.querySelector<HTMLInputElement>('input[type="password"]')!;
    fireEvent.change(input, { target: { value: "sk-manual-secret" } });
    fireEvent.keyDown(input, { key: "Enter" });

    expect(await screen.findByText("gpt-5.6-terra")).toBeInTheDocument();
    expect(invokeMock).toHaveBeenCalledWith("sub2api_prepare_manual", { apiKey: "sk-manual-secret" });
    const confirm = screen.getByRole("button", { name: /^(Verify and configure|验证并配置|sub2api\.confirm)$/ });
    fireEvent.click(confirm);
    await waitFor(() => expect(invokeMock).toHaveBeenCalledWith("sub2api_apply_setup", { clientVersion: "0.1.0" }));
  });

  it("accepts a queued deep-link ticket through the isolated setup event", async () => {
    render(<Sub2APIQuickStart onAdvanced={() => undefined} />);
    await waitFor(() => expect(setupListener).toBeDefined());
    await act(async () => setupListener?.({ payload: { ticket: "a".repeat(43) } }));
    await waitFor(() => expect(invokeMock).toHaveBeenCalledWith("sub2api_prepare_ticket", { ticket: "a".repeat(43) }));
  });

  it("warns when local setup succeeds but website confirmation is pending", async () => {
    invokeMock.mockImplementation((command: string) => {
      if (command === "sub2api_setup_status") return Promise.resolve(false);
      if (command === "sub2api_take_queued_ticket") return Promise.resolve(null);
      if (command === "sub2api_prepare_manual") return Promise.resolve(preview);
      if (command === "sub2api_apply_setup") return Promise.resolve({ configured: true, verified: true, server_confirmed: false, backup_directory: "/tmp/backup", restart_required: true });
      return Promise.reject(new Error(`unexpected command ${command}`));
    });
    const { container } = render(<Sub2APIQuickStart onAdvanced={() => undefined} />);
    const input = container.querySelector<HTMLInputElement>('input[type="password"]')!;
    fireEvent.change(input, { target: { value: "sk-manual-secret" } });
    fireEvent.keyDown(input, { key: "Enter" });
    fireEvent.click(await screen.findByRole("button", { name: /^(Verify and configure|验证并配置|sub2api\.confirm)$/ }));

    expect(await screen.findByText(/website could not be notified|暂时无法通知网站|sub2api\.confirmPending/)).toBeInTheDocument();
  });
});
