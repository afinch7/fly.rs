import { FlyModuleService, FlyModuleServiceInstaceConfig, PathLike, AbstractFileSystem } from "../../node_modules/fly-module-services/src/index";
import { URL } from "../url";
import { ReadFileUrlResponse } from "../service_runtimes";

function getDefaultUrl(runtimeId: string): string {
    const runtimeInfo = serviceRuntimes.getRuntimeInfo(runtimeId);
    return runtimeInfo.default_module_url;
}

class FilesystemWithCaching implements AbstractFileSystem {
    private readonly fileMap: Map<string, ReadFileUrlResponse> = new Map();

    constructor(
        readonly runtimeId: string,
    ) {

    }

    exists(path: PathLike) {
        let pathUrl: URL;
        if (typeof path !== "string") {
            pathUrl = new URL(path.toString());
        } else {
            pathUrl = new URL(path); 
        }
        if (this.fileMap.has(pathUrl.toString())) {
            return this.fileMap.get(pathUrl.toString()).exists;
        }
        const fileInfo = serviceRuntimes.readFileUrl(pathUrl);
        this.fileMap.set(pathUrl.toString(), fileInfo);
        return fileInfo.exists;
    }

    read(path: any) {
        let pathUrl: URL;
        if (typeof path !== "string") {
            pathUrl = new URL(path.toString());
        } else {
            pathUrl = new URL(path); 
        }
        let fileInfo: ReadFileUrlResponse;
        if (this.fileMap.has(pathUrl.toString())) {
            fileInfo = this.fileMap.get(pathUrl.toString());
        } else {
            fileInfo = serviceRuntimes.readFileUrl(pathUrl);
            this.fileMap.set(pathUrl.toString(), fileInfo);
        }
        if (fileInfo.exists) {
            return fileInfo.content;
        } else {
            throw new Error("File doesn't exist.");
        }
    }
}

const flyModuleService = new FlyModuleService({
    gernerateInstanceConfig(runtimeId: string): FlyModuleServiceInstaceConfig {
        return {
            defaultUrl: getDefaultUrl(runtimeId),
            filesystem: new FilesystemWithCaching(runtimeId),
        };
    },
    defaultUrl: "file:///app.js",
});

addEventListener("serve", (event) => {
    
});