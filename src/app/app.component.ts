import { DatePipe, DecimalPipe } from "@angular/common";
import { Component } from "@angular/core";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";

type FileCategory = "Images" | "Documents" | "Videos" | "Audio" | "Archives" | "Installers" | "Other";

interface FilePreview {
  name: string;
  path: string;
  extension: string;
  size: number;
  modifiedTime: number;
  category: FileCategory;
  proposedDestination: string;
  willMove: boolean;
}

const AGGRESSIVENESS_LABELS = ["", "Light tidy", "Balanced", "Clean sweep"] as const;

@Component({
  selector: "app-root",
  imports: [DatePipe, DecimalPipe],
  templateUrl: "./app.component.html",
  styleUrl: "./app.component.css",
})
export class AppComponent {
  selectedDirectory = "";
  files: FilePreview[] = [];
  errorMessage = "";
  isLoading = false;
  aggressiveness = 2;

  get aggressivenessLabel(): string {
    return AGGRESSIVENESS_LABELS[this.aggressiveness] ?? AGGRESSIVENESS_LABELS[2];
  }

  async selectDirectory(): Promise<void> {
    this.errorMessage = "";
    try {
      const selected = await open({ directory: true, multiple: false });
      if (!selected) return;

      this.selectedDirectory = selected;
      this.files = [];
      this.isLoading = true;
      await this.scanDirectory();
    } catch (error: unknown) {
      this.errorMessage = this.describeError(error);
    } finally {
      this.isLoading = false;
    }
  }

  async changeAggressiveness(event: Event): Promise<void> {
    this.aggressiveness = Number((event.target as HTMLInputElement).value);
    if (this.selectedDirectory) await this.scanDirectory();
  }

  private async scanDirectory(): Promise<void> {
    this.errorMessage = "";
    this.isLoading = true;
    try {
      this.files = await invoke<FilePreview[]>("scan_directory", {
        directory: this.selectedDirectory,
        aggressiveness: this.aggressiveness,
      });
    } catch (error: unknown) {
      this.files = [];
      this.errorMessage = this.describeError(error);
    } finally {
      this.isLoading = false;
    }
  }

  private describeError(error: unknown): string {
    return typeof error === "string"
      ? error
      : error instanceof Error
        ? error.message
        : "The directory could not be scanned. Please try another folder.";
  }
}
