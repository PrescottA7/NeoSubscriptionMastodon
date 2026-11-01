import { configureStore } from "@reduxjs/toolkit";

import { authReducer } from "./authSlice";
import { catalogReducer } from "./catalogSlice";
import { noticesReducer } from "./noticesSlice";

export const store = configureStore({
  reducer: {
    auth: authReducer,
    catalog: catalogReducer,
    notices: noticesReducer,
  },
});

export type RootState = ReturnType<typeof store.getState>;
export type AppDispatch = typeof store.dispatch;
