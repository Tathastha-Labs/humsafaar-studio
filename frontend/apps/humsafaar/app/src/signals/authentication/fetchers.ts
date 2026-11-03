import { authentication } from "./authentication";
import { updateAuthStatus } from "./utilities";
import { AUTH_STATUS } from "~/enums";

// There is no account: this is a single-user app that keeps everything on this computer.
// The status exists only so code that reads it has a settled value.
export const persistLogin = async () => {
  if (authentication.status.value !== AUTH_STATUS.INIT) {
    return;
  }
  updateAuthStatus(AUTH_STATUS.LOGGED_IN);
};
